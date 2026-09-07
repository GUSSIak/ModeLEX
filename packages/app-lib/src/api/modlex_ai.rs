//! packages/app-lib/src/api/modlex_ai.rs
//! ModLEX AI agent — chat + tool-calling.
//! Phase 1: read-only "tier 0" tools. Phase 2: confirm-gated state-changing
//! ("tier Confirm") tools — install/remove/toggle/change-version a mod, set
//! instance Java/memory, install a Java runtime.
//!
//! Default backend is z.ai's OpenAI-compatible chat completions API (a pool
//! of built-in free-tier keys, ZAI_API_KEY_1..N, GLM models, baked in at
//! build time from packages/app-lib/.env, plus the user's own z.ai key from
//! Settings.modlex_ai_api_key if the whole pool is exhausted) — falls back to
//! Cloudflare Workers AI (CLOUDFLARE_ACCOUNT_ID_N/CLOUDFLARE_API_TOKEN_N
//! pairs, gpt-oss-120b) if exhausted, and only as a LAST resort to Groq (its
//! own key pool, GROQ_API_KEY_1..N — no personal-key fallback here anymore,
//! see 2026-09-06 note below) — Groq moved to last-resort 2026-09-05 (see
//! memory project_modlex_ai_agent), kept only for when the other two are
//! both unreachable. All three happen to expose an OpenAI-wire-compatible
//! chat completions endpoint (confirmed live 2026-08-29), so
//! `try_openai_compatible_call` serves all three with the same
//! request/response structs.
//!
//! 2026-09-06: two changes. (1) The personal-key fallback moved from Groq to
//! z.ai — z.ai is the primary tier now, so a personal key is actually useful
//! there (a personal Groq key sat behind the ENTIRE z.ai+Cloudflare+Groq-pool
//! chain and was rarely ever reached). (2) Each pool is shuffled right before
//! the fallback loop in `chat_raw` (not inside the `*_key_pool()` getters
//! themselves, which stay in stable declared order for enumeration/ping
//! purposes) — every install trying keys in the same 1→2→3→... order meant
//! the low-numbered keys always absorbed the first hit of rate limiting
//! (z.ai) or ran out of Workers AI "neurons" first (Cloudflare) across the
//! whole user base, while later keys sat idle. Randomizing spreads load.
//!
//! Tool-calling design (see `agent_step`/`agent_resume`/`agent_loop`): the
//! WHOLE conversation history (including tool_calls/tool-result messages)
//! round-trips through the frontend on every turn — there is no server-side
//! session state, mirroring the existing `begin_login`/`finish_login`
//! stateless-token pattern in `minecraft_auth.rs`. Every tool has a "tier":
//! `Auto` tools execute immediately with no confirmation (read-only, scoped
//! to launcher/mod data); `Confirm` tools (Phase 2) stop the loop and return
//! `AgentStepOutcome::PendingConfirmation` — the frontend must show a
//! confirm/deny UI and call `agent_resume` with the user's decision before
//! anything actually runs. `AskFirst` is reserved (read, but leaves the
//! machine — e.g. hardware info) — nothing registers it yet.

use rand::seq::SliceRandom;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::Path;
use std::time::Duration;

const GROQ_CHAT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const GROQ_MODELS_URL: &str = "https://api.groq.com/openai/v1/models";
/// Primary provider — tried first (pool, then the user's own z.ai key from
/// Settings.modlex_ai_api_key). z.ai's chat completions API is
/// wire-compatible with Groq's (confirmed live 2026-08-29: same
/// `tools`/`tool_calls` shape, same message shape — the only extra field,
/// `reasoning_content`, is silently ignored since `AiChatMessage` doesn't
/// declare it). Always uses `ZAI_DEFAULT_MODEL`, ignoring the caller's
/// `model` param — z.ai's catalog is unrelated to Groq's, so
/// Settings.modlex_ai_model (a Groq model id) doesn't apply here.
const ZAI_CHAT_URL: &str = "https://api.z.ai/api/paas/v4/chat/completions";
const ZAI_DEFAULT_MODEL: &str = "glm-4.5-flash";
/// Third-provider fallback — Cloudflare Workers AI's OpenAI-compatible
/// endpoint (confirmed live 2026-08-29, same wire shape again — tool calling
/// works, extra `reasoning`/`reasoning_content` fields ignored same as z.ai).
/// Unlike Groq/z.ai, a Cloudflare "key" is a (account_id, api_token) PAIR —
/// the account_id is baked into the URL itself, not sent as a header — so
/// `cloudflare_key_pool()` returns pairs instead of bare strings.
const CLOUDFLARE_DEFAULT_MODEL: &str = "@cf/openai/gpt-oss-120b";
/// Default/fallback when Settings.modlex_ai_model is empty (fresh installs —
/// see the migration default). 20b, not 120b — 120b was burning through the
/// shared free-key pool's quota too fast for real back-and-forth chats
/// (confirmed live 2026-08-28: pool exhausted after ~3 exchanges on a single
/// test key). Groq's lineup changes over time (checked live via
/// GET /openai/v1/models); the real source of truth for what's selectable
/// is `list_models`, not this constant.
pub const DEFAULT_GROQ_MODEL: &str = "openai/gpt-oss-20b";
/// Model ids that exist in the Groq catalog but aren't general-purpose text
/// chat models (audio/TTS/safety-classifier/etc.) — filtered out of
/// `list_models` so the picker isn't cluttered with models that will just
/// error out on a chat request. Substring match against the id.
const NON_CHAT_MODEL_MARKERS: &[&str] = &[
    "whisper",
    "orpheus",
    "guard",
    "safeguard",
    "tts",
];
/// Models that reject custom user-defined `tools` entirely — they only
/// orchestrate Groq's own built-in tools (web search/code execution) and
/// error out if you attach a `tools` array. Verified against Groq's
/// tool-use docs on 2026-08-28. `agent_step` falls back to plain chat
/// (no tool-calling) for these instead of attaching tool definitions.
const NO_CUSTOM_TOOLS_MODELS: &[&str] = &["groq/compound", "groq/compound-mini"];
/// How many GROQ_API_KEY_N env slots we look for at build time. Bump this if
/// the pool ever needs to exceed 20 keys.
const MAX_POOL_KEYS: usize = 20;
/// Hard cap on consecutive tool-call hops within a single `agent_step`/
/// `agent_resume` call (each gets its OWN fresh budget — a Confirm-tier stop
/// doesn't carry the count over), so a model that never stops calling tools
/// can't loop forever burning the shared key pool. Hitting this returns a
/// normal `Reply`, not an error. Raised from 8, then 14 — a real
/// multi-dependency install (search + version-lookup per dependency, ×2+
/// mods) can burn 8-14 hops just on read-only investigation before ever
/// reaching a Confirm call, and with auto-confirm on (no pause between
/// mods) a "fix 10 mods" task needs a much bigger budget in ONE call.
/// User-configurable (Settings.modlex_ai_max_hops) — this is just the
/// fallback/clamp default, see `resolve_max_hops`.
const DEFAULT_MAX_HOPS: usize = 24;
/// Верхняя граница на случай, если в настройках оказалось что-то абсурдное
/// (0, отрицательное после будущих изменений типа, случайно вбитые нули) —
/// не даём разово сжечь весь пул ключей одним зависшим диалогом.
const MAX_TOOL_HOPS_CEILING: usize = 60;

fn resolve_max_hops(configured: u32) -> usize {
    if configured == 0 {
        DEFAULT_MAX_HOPS
    } else {
        (configured as usize).min(MAX_TOOL_HOPS_CEILING)
    }
}

const BASE_SYSTEM_PROMPT: &str = "\
Ты — встроенный ИИ-помощник лаунчера ModLEX (форк Modrinth App для установки \
модов и модпаков Minecraft). Помогаешь пользователю разбираться с ошибками \
запуска, крашами, выбором модов/сборок и общими вопросами про лаунчер.

У тебя есть инструменты для чтения данных лаунчера (список инстанций, \
установленный контент, краш-логи, поиск модов на Modrinth, версии проектов, \
установленные Java, версии загрузчика модов) — используй их, чтобы отвечать \
по существу конкретной ситуации пользователя, а не гадать. Если не понятно, \
о какой инстанции речь, сначала вызови list_instances, а не спрашивай \
пользователя об ID напрямую.

ВАЖНО про память между сессиями: история ЭТОГО диалога пропадёт при \
перезапуске лаунчера, но save_note/list_notes — нет, они хранятся отдельно. \
Если разговор похож на продолжение прошлой работы (пользователь ссылается \
на что-то, что вы вроде уже обсуждали, или просто в начале диалога по \
конкретной инстанции) — вызови list_notes, прежде чем переспрашивать то, \
что могло уже быть выяснено раньше. Когда делаешь что-то, что имеет смысл \
помнить в следующий раз (поставил/убрал моды, выяснил предпочтения \
пользователя, диагностировал причину краша) — запиши через save_note.

ВАЖНО: не путай загрузчик модов (Fabric Loader/Forge/Quilt/NeoForge — это \
СВОЙСТВО ИНСТАНЦИИ, версия самого загрузчика) с одноимённым/похожим модом \
на Modrinth (например \"Fabric API\" — это отдельная библиотека-мод, у неё \
СВОЯ независимая нумерация версий). Если краш связан с тем, что версия \
ЗАГРУЗЧИКА слишком старая/новая для установленных модов — используй \
get_loader_versions и set_instance_loader, а НЕ install_mod_version с \
проектом \"Fabric API\"/\"Forge\" и т.п. — это не решит проблему.

ВАЖНО про поиск проектов: search_mods может найти НЕСКОЛЬКО разных проектов \
с похожим или даже тем же названием — например, оригинальный мод одного \
автора и отдельный неофициальный порт под другой загрузчик от ДРУГОГО \
автора (разные project_id, разные страницы, разный changelog — это НЕ один \
и тот же мод, просто одинаково называется). Если по ходу разговора \
пришлось взять другой проект вместо того, что обсуждался изначально \
(например, у оригинала нет версии под нужный загрузчик инстанции) — явно \
скажи об этом (другой автор/порт), не выдавай его молча за тот же мод, \
который спрашивал пользователь. И отдельно: результат get_project_versions \
уже включает РЕАЛЬНОЕ поле changelog по каждой версии — прежде чем писать \
пользователю \"changelog не указан/отсутствует\", проверь, что это поле в \
фактически полученном результате вызова действительно пустое, а не \
предполагай это заранее.

ВАЖНО про установку модов: install_mod_version и install_mod_versions САМИ \
резолвят и ставят обязательные зависимости мода — НЕ нужно отдельно искать \
и ставить зависимость как ещё один мод (например, для аддона к Create не \
нужно отдельно искать и ставить сам Create — это произойдёт автоматически). \
Если нужно поставить БОЛЬШЕ ОДНОГО мода — используй install_mod_versions \
(один вызов, один список \"мод + версия\" на все сразу, одно подтверждение \
пользователя), а НЕ install_mod_version по кругу на каждый мод отдельно — \
так экономятся вызовы инструментов и не упираешься в лимит хопов на \
середине сборки.

У тебя также есть инструменты, которые РЕАЛЬНО МЕНЯЮТ лаунчер (установить/ \
удалить/включить-выключить/сменить версию мода, сменить загрузчик/версию \
игры инстанции, назначить Java или память инстанции, поставить Java, \
запустить игру, создать или переименовать инстанцию). Перед \
вызовом такого инструмента сначала КОРОТКО объясни текстом, что именно \
собираешься сделать и зачем — пользователь в любом случае увидит карточку с \
подтверждением и должен явно её принять, прежде чем действие выполнится, но \
объяснение помогает понять, на что он соглашается. Никогда не утверждай, что \
уже что-то изменил, если инструмент ещё не вернул результат (пользователь \
мог отклонить действие).

ВАЖНО про границу \"проверить/рассказать\" и \"сделать\": просьба проверить, \
посмотреть, найти проблему или описать состояние (например \"проверь мои \
инстанции на устаревшие моды и опиши каждую\") — это НЕ разрешение сразу же \
исправлять найденное. Для такой просьбы используй только читающие \
инструменты (list_installed_content, get_project_versions и т.п.) и вынеси \
находки в ответ текстом; state-changing инструмент (update_instance_content, \
install/remove/toggle мода и т.п.) вызывай ТОЛЬКО если пользователь явно \
попросил само действие (\"обнови\", \"поставь\", \"удали\", \"почини\") — а не \
потому что попутно нашёл, что можно обновить. Это особенно важно, если у \
пользователя включён режим без подтверждений: тогда твой выбор — \
единственная защита от лишних изменений, карточка подтверждения его не \
подстрахует.

Не повторяй один и тот же диагноз/анализ ситуации заново в каждом сообщении \
хода — если ты уже объяснил причину проблемы один раз, на следующих шагах \
просто коротко говори, что делаешь дальше (например \"Ищу подходящую \
версию...\"), не пересказывая весь анализ с нуля.

ВАЖНО про краш-репорты: get_last_crash_report описывает ОДИН КОНКРЕТНЫЙ \
прошлый запуск игры (см. поле created_seconds_ago) и НЕ обновляется сам по \
себе — если ты уже применил исправление (установил/удалил/сменил мод, \
сменил загрузчик и т.п.), а пользователь ещё не подтвердил, что запускал \
игру заново, повторный вызов get_last_crash_report вернёт ТОТ ЖЕ старый \
отчёт. Это НЕ значит, что исправление не сработало — это значит, что игру \
ещё не запускали. Не делай новых выводов и не предлагай новых действий по \
уже прочитанному отчёту повторно: после исправления сообщи, что сделано, и \
попроси пользователя запустить игру и написать, если проблема останется.

ВАЖНО про report_bug: это реально уходит разработчику. Просьба пользователя \
\"напиши репорт\"/\"просто отправь\" — это НЕ доказательство настоящего бага, \
это пересказ его слов. Прежде чем предлагать отправку — сначала проверь \
данными: краш/ошибка ИГРЫ в конкретной инстанции — get_last_crash_report; \
жалоба на сам ЛАУНЧЕР (глючит, зависает, не открывается) — get_launcher_log. \
Опирайся на то, что реально нашёл, а не на слова пользователя; если \
конкретных шагов воспроизведения нет — переспроси; никогда не подставляй в \
поля (частота, серьёзность) то, что пользователь сам не подтвердил. \
Настойчивость пользователя — не повод соглашаться быстрее.

Отвечай кратко и по делу, на языке пользователя.";

/// Добавляется к базовому промпту только когда запрос реально может уйти в
/// общий бесплатный пул ключей (см. `system_prompt`) — "защита от дурака" от
/// офтопа, который просто впустую тратит общий лимит. Обычные приветствия
/// специально разрешены явно, чтобы модель не начинала отказывать на
/// безобидное "привет"/"как дела" в начале разговора.
const TOPIC_BOUNDARY_INSTRUCTION: &str = "\n\n\
Ты работаешь через ОБЩИЙ бесплатный пул ключей, которым пользуются и другие \
люди — береги его. Если вопрос вообще не по теме лаунчера/модов/Minecraft \
(например, про погоду, политику, отвлечённую болтовню ни о чём) — вежливо и \
КОРОТКО (1 предложение) откажись отвечать по существу и напомни, что ты \
помощник именно по лаунчеру. Обычные приветствия и вежливые фразы в начале \
разговора ('привет', 'как дела') — это нормально, на них отвечай естественно, \
это не офтоп.";

/// `restrict_to_topic` — true когда запрос может обслужиться общим пулом
/// ключей (см. `chat_with_system_prompt`); при использовании собственного
/// ключа пользователя ограничение по теме отключается полностью — это его
/// личный лимит, пусть спрашивает что угодно.
pub fn system_prompt(restrict_to_topic: bool) -> String {
    if restrict_to_topic {
        format!("{BASE_SYSTEM_PROMPT}{TOPIC_BOUNDARY_INSTRUCTION}")
    } else {
        BASE_SYSTEM_PROMPT.to_string()
    }
}

fn groq_key_pool() -> Vec<&'static str> {
    // option_env! must be called with a literal, so this can't be a loop —
    // unrolled up to MAX_POOL_KEYS. Add rows here if that constant grows.
    let slots: [Option<&'static str>; MAX_POOL_KEYS] = [
        option_env!("GROQ_API_KEY_1"),
        option_env!("GROQ_API_KEY_2"),
        option_env!("GROQ_API_KEY_3"),
        option_env!("GROQ_API_KEY_4"),
        option_env!("GROQ_API_KEY_5"),
        option_env!("GROQ_API_KEY_6"),
        option_env!("GROQ_API_KEY_7"),
        option_env!("GROQ_API_KEY_8"),
        option_env!("GROQ_API_KEY_9"),
        option_env!("GROQ_API_KEY_10"),
        option_env!("GROQ_API_KEY_11"),
        option_env!("GROQ_API_KEY_12"),
        option_env!("GROQ_API_KEY_13"),
        option_env!("GROQ_API_KEY_14"),
        option_env!("GROQ_API_KEY_15"),
        option_env!("GROQ_API_KEY_16"),
        option_env!("GROQ_API_KEY_17"),
        option_env!("GROQ_API_KEY_18"),
        option_env!("GROQ_API_KEY_19"),
        option_env!("GROQ_API_KEY_20"),
    ];
    slots.into_iter().flatten().filter(|k| !k.is_empty()).collect()
}

fn zai_key_pool() -> Vec<&'static str> {
    let slots: [Option<&'static str>; MAX_POOL_KEYS] = [
        option_env!("ZAI_API_KEY_1"),
        option_env!("ZAI_API_KEY_2"),
        option_env!("ZAI_API_KEY_3"),
        option_env!("ZAI_API_KEY_4"),
        option_env!("ZAI_API_KEY_5"),
        option_env!("ZAI_API_KEY_6"),
        option_env!("ZAI_API_KEY_7"),
        option_env!("ZAI_API_KEY_8"),
        option_env!("ZAI_API_KEY_9"),
        option_env!("ZAI_API_KEY_10"),
        option_env!("ZAI_API_KEY_11"),
        option_env!("ZAI_API_KEY_12"),
        option_env!("ZAI_API_KEY_13"),
        option_env!("ZAI_API_KEY_14"),
        option_env!("ZAI_API_KEY_15"),
        option_env!("ZAI_API_KEY_16"),
        option_env!("ZAI_API_KEY_17"),
        option_env!("ZAI_API_KEY_18"),
        option_env!("ZAI_API_KEY_19"),
        option_env!("ZAI_API_KEY_20"),
    ];
    slots.into_iter().flatten().filter(|k| !k.is_empty()).collect()
}

/// Returns (account_id, api_token) pairs — an entry only counts if BOTH env
/// vars for that slot are present and non-empty (a lone account_id or lone
/// token is useless on its own).
fn cloudflare_key_pool() -> Vec<(&'static str, &'static str)> {
    let account_ids: [Option<&'static str>; MAX_POOL_KEYS] = [
        option_env!("CLOUDFLARE_ACCOUNT_ID_1"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_2"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_3"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_4"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_5"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_6"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_7"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_8"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_9"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_10"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_11"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_12"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_13"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_14"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_15"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_16"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_17"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_18"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_19"),
        option_env!("CLOUDFLARE_ACCOUNT_ID_20"),
    ];
    let api_tokens: [Option<&'static str>; MAX_POOL_KEYS] = [
        option_env!("CLOUDFLARE_API_TOKEN_1"),
        option_env!("CLOUDFLARE_API_TOKEN_2"),
        option_env!("CLOUDFLARE_API_TOKEN_3"),
        option_env!("CLOUDFLARE_API_TOKEN_4"),
        option_env!("CLOUDFLARE_API_TOKEN_5"),
        option_env!("CLOUDFLARE_API_TOKEN_6"),
        option_env!("CLOUDFLARE_API_TOKEN_7"),
        option_env!("CLOUDFLARE_API_TOKEN_8"),
        option_env!("CLOUDFLARE_API_TOKEN_9"),
        option_env!("CLOUDFLARE_API_TOKEN_10"),
        option_env!("CLOUDFLARE_API_TOKEN_11"),
        option_env!("CLOUDFLARE_API_TOKEN_12"),
        option_env!("CLOUDFLARE_API_TOKEN_13"),
        option_env!("CLOUDFLARE_API_TOKEN_14"),
        option_env!("CLOUDFLARE_API_TOKEN_15"),
        option_env!("CLOUDFLARE_API_TOKEN_16"),
        option_env!("CLOUDFLARE_API_TOKEN_17"),
        option_env!("CLOUDFLARE_API_TOKEN_18"),
        option_env!("CLOUDFLARE_API_TOKEN_19"),
        option_env!("CLOUDFLARE_API_TOKEN_20"),
    ];
    account_ids
        .into_iter()
        .zip(api_tokens)
        .filter_map(|(id, token)| match (id, token) {
            (Some(id), Some(token)) if !id.is_empty() && !token.is_empty() => {
                Some((id, token))
            }
            _ => None,
        })
        .collect()
}

fn model_supports_custom_tools(model: &str) -> bool {
    !NO_CUSTOM_TOOLS_MODELS
        .iter()
        .any(|m| model.eq_ignore_ascii_case(m))
}

// ── Модель сообщений ─────────────────────────────────────────────────────────

/// Один вызов инструмента моделью. Форма на проводе (что реально уходит/
/// приходит в JSON, и в Groq, и во фронт) — вложенная OpenAI-совместимая
/// `{id, type: "function", function: {name, arguments}}` (см. `Serialize`/
/// `Deserialize` ниже); в самом Rust-коде и в TS на фронте она плоская
/// `{id, name, arguments}` — так удобнее и там, и там.
#[derive(Debug, Clone)]
pub struct AiToolCall {
    pub id: String,
    pub name: String,
    /// Сырой JSON-текст аргументов, как прислала модель — не распарсенный
    /// заранее, потому что схема у каждого инструмента своя.
    pub arguments: String,
}

#[derive(Serialize, Deserialize)]
struct AiToolCallWireFn {
    name: String,
    arguments: String,
}

#[derive(Serialize, Deserialize)]
struct AiToolCallWire {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: AiToolCallWireFn,
}

impl Serialize for AiToolCall {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AiToolCallWire {
            id: self.id.clone(),
            kind: "function".to_string(),
            function: AiToolCallWireFn {
                name: self.name.clone(),
                arguments: self.arguments.clone(),
            },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AiToolCall {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = AiToolCallWire::deserialize(deserializer)?;
        Ok(AiToolCall {
            id: wire.id,
            name: wire.function.name,
            arguments: wire.function.arguments,
        })
    }
}

/// Одно сообщение диалога. Эта же структура — и формат запроса/ответа Groq
/// (уже ровно то, что Groq ждёт в `messages`), и формат истории, которую
/// видит и хранит фронт между ходами (стейта на сервере нет вообще —
/// фронт присылает обратно весь массив целиком на каждый ход).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiChatMessage {
    /// "system" | "user" | "assistant" | "tool"
    pub role: String,
    /// `None` допустим у ассистента, когда он только вызывает инструменты,
    /// ничего не говоря текстом (Groq/z.ai отдают такое сообщение с
    /// content: null и тихо принимают его же обратно). Cloudflare Workers AI
    /// — нет: его схема валидации для /ai/v1/chat/completions требует
    /// content ИМЕННО строкой на КАЖДОМ сообщении, включая
    /// tool_calls-only — что null, что вовсе опущенное поле одинаково
    /// валятся 400-й с "'string' not in 'null'"/"required properties ...
    /// are 'role,content'" (воспроизведено и подтверждено вручную curl'ом
    /// 2026-09-08 — это и была причина "ошибки ожидания" в реальном
    /// диалоге через Cloudflare на втором ходе, после первого вызова
    /// инструмента). Раз простая пустая строка одинаково устраивает все три
    /// провайдера (проверено), сериализуем None как "" всегда, а не
    /// пытаемся различать провайдеров здесь.
    #[serde(default, serialize_with = "serialize_content_as_string")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AiToolCall>>,
    /// Обязательно у сообщений с role="tool" — какой именно вызов это ответ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

fn serialize_content_as_string<S>(
    content: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(content.as_deref().unwrap_or(""))
}

impl AiChatMessage {
    fn system(text: String) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(text),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn tool_result(tool_call_id: String, content: String) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }
}

// ── Groq wire types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GroqToolFunctionWire {
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
}

#[derive(Serialize)]
struct GroqToolWire {
    #[serde(rename = "type")]
    kind: &'static str,
    function: GroqToolFunctionWire,
}

fn groq_tool(
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
) -> GroqToolWire {
    GroqToolWire {
        kind: "function",
        function: GroqToolFunctionWire {
            name,
            description,
            parameters,
        },
    }
}

#[derive(Serialize)]
struct GroqRequest<'a> {
    model: &'a str,
    messages: &'a [AiChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [GroqToolWire]>,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Deserialize)]
struct GroqChoice {
    // Groq's assistant message shape is exactly `AiChatMessage`'s wire
    // shape (role/content/tool_calls) — no separate type needed.
    message: AiChatMessage,
}

fn groq_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        // A short connect timeout, separate from the overall 60s response
        // timeout — only affects TCP/TLS handshake time, never cuts off an
        // already-connected request that's just slow to respond, so it's
        // free resilience: a genuinely unreachable endpoint now fails (and,
        // per try_openai_compatible_call, falls through to the next key)
        // in ~5s instead of eating up to 60s per dead key in the pool.
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("Groq reqwest client")
}

/// Общая функция для ЛЮБОГО OpenAI-wire-совместимого чат-провайдера — сейчас
/// используется и для Groq, и для z.ai, отличаются только `url`/`api_key`/
/// `model`. Формат запроса и разбор ответа идентичны (см. `AiChatMessage`'s
/// `Deserialize` — лишние поля вроде z.ai-шного `reasoning_content` просто
/// игнорируются serde).
/// `reqwest::Error` из `.send()` для реального сетевого сбоя (нет связи, DNS,
/// таймаут) даёт технический английский текст вроде "error trying to
/// connect: dns error: ..." — пользователю это не понятно и не помогает.
/// Подменяем на короткое понятное сообщение по-русски; всё остальное (ошибки
/// HTTP-статуса, разбора JSON) обрабатывается отдельно и сюда не попадает.
fn describe_request_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "Сервер ИИ не отвечает — истекло время ожидания. Попробуйте ещё раз.".to_string()
    } else if e.is_connect() {
        "Не удалось подключиться к серверу ИИ — проверьте подключение к интернету.".to_string()
    } else {
        e.to_string()
    }
}

async fn try_openai_compatible_call(
    url: &str,
    api_key: &str,
    model: &str,
    messages: &[AiChatMessage],
    tools: Option<&[GroqToolWire]>,
) -> crate::Result<Option<AiChatMessage>> {
    let client = groq_client();
    let body = GroqRequest {
        model,
        messages,
        tools,
    };

    // 2026-09-08: a connection failure/timeout on THIS key used to `?`
    // straight out of the whole function, which chat_raw's caller treated as
    // a hard, chain-aborting error — one dead/unreachable key took down the
    // ENTIRE fallback chain, never even trying the next key, let alone the
    // next provider. Confirmed live: 3 of 5 pooled z.ai keys were fully
    // unreachable (connection timeout) from this network at the time, and
    // every one of them was killing requests outright instead of falling
    // through to a healthy key or to Cloudflare — this is almost certainly
    // what "всегда ошибка ожидания" was actually seeing. A network-level
    // failure says nothing about whether THIS key is bad, so treat it the
    // same as rate-limiting: `Ok(None)` tells the caller to try the next key.
    let response = match client.post(url).bearer_auth(api_key).json(&body).send().await {
        Ok(response) => response,
        Err(e) if e.is_timeout() || e.is_connect() => return Ok(None),
        Err(e) => {
            return Err(crate::ErrorKind::OtherError(describe_request_error(&e)).as_error());
        }
    };

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        || response.status().is_server_error()
    {
        // Rate-limited, or a transient server-side error (5xx) — neither is
        // evidence the KEY itself is bad, so try the next one in the pool
        // rather than aborting the whole chain over what's likely a
        // temporary blip on this one endpoint.
        return Ok(None);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(crate::ErrorKind::OtherError(format!(
            "Ошибка API ({url}) {status}: {text}"
        ))
        .as_error());
    }

    // Читаем как текст, а не сразу .json() — нужен сырой текст для
    // диагностики ниже (см. пометку про пустые имена тулов), а Response::json()
    // сам по себе не даёт доступа к телу постфактум, если разбор не упал.
    let raw_body = response
        .text()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;
    let parsed: GroqResponse = serde_json::from_str(&raw_body).map_err(|e| {
        crate::ErrorKind::OtherError(format!("{e}; сырой ответ: {raw_body}")).as_error()
    })?;

    let message = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError("API response had no choices".to_string())
                .as_error()
        })?;

    Ok(Some(message))
}

/// `provider` — "auto" (default: z.ai pool → Cloudflare pool → Groq pool →
/// Groq user key, in that order — see memory project_modlex_ai_agent for why
/// Groq moved to last resort) or a forced single provider ("groq" | "zai" |
/// "cloudflare") that skips straight to that tier and never falls through to
/// the others. Added so a specific provider can be isolated for
/// testing/debugging. Unknown/empty values are treated as "auto".
fn wants_provider(provider: &str, name: &str) -> bool {
    provider.is_empty() || provider.eq_ignore_ascii_case("auto") || provider.eq_ignore_ascii_case(name)
}

/// z.ai (pool, then the user's own key) first, then Cloudflare, and Groq
/// (pool only) as a last resort — both z.ai/Cloudflare use hardcoded models;
/// Settings.modlex_ai_model only applies to Groq. Shared core for both plain
/// chat and the tool-calling loop, differing only in whether `tools` is
/// passed. `provider` can force one specific tier instead of the full
/// fallback chain — see `wants_provider`. Each pool is shuffled right here,
/// per call, so many concurrently-running installs don't all hammer the same
/// low-numbered key first (see 2026-09-06 module-doc note).
async fn chat_raw(
    messages: &[AiChatMessage],
    user_key: Option<String>,
    model: &str,
    tools: Option<&[GroqToolWire]>,
    provider: &str,
) -> crate::Result<AiChatMessage> {
    if wants_provider(provider, "zai") {
        let mut pool = zai_key_pool();
        pool.shuffle(&mut rand::thread_rng());
        for key in &pool {
            match try_openai_compatible_call(
                ZAI_CHAT_URL,
                key,
                ZAI_DEFAULT_MODEL,
                messages,
                tools,
            )
            .await
            {
                Ok(Some(message)) => return Ok(message),
                Ok(None) => continue, // rate-limited on this key, try next
                Err(e) => return Err(e),
            }
        }

        if let Some(key) = user_key.as_deref().filter(|k| !k.is_empty()) {
            if let Some(message) =
                try_openai_compatible_call(ZAI_CHAT_URL, key, ZAI_DEFAULT_MODEL, messages, tools)
                    .await?
            {
                return Ok(message);
            }
        }
    }

    if wants_provider(provider, "cloudflare") {
        let mut pool = cloudflare_key_pool();
        pool.shuffle(&mut rand::thread_rng());
        for (account_id, token) in &pool {
            let url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1/chat/completions"
            );
            match try_openai_compatible_call(
                &url,
                token,
                CLOUDFLARE_DEFAULT_MODEL,
                messages,
                tools,
            )
            .await
            {
                Ok(Some(message)) => return Ok(message),
                Ok(None) => continue, // rate-limited on this key, try next
                Err(e) => return Err(e),
            }
        }
    }

    if wants_provider(provider, "groq") {
        let mut pool = groq_key_pool();
        pool.shuffle(&mut rand::thread_rng());
        for key in &pool {
            match try_openai_compatible_call(GROQ_CHAT_URL, key, model, messages, tools).await {
                Ok(Some(message)) => return Ok(message),
                Ok(None) => continue, // rate-limited on this key, try next
                Err(e) => return Err(e), // a real error (bad key, network, etc.)
            }
        }
    }

    Err(crate::ErrorKind::OtherError(format!(
        "Все встроенные бесплатные ключи (провайдер: {}) сейчас исчерпаны, а \
         свой ключ не указан. Добавьте свой бесплатный ключ z.ai в настройках, \
         чтобы продолжить пользоваться ИИ-агентом.",
        if provider.is_empty() { "auto" } else { provider }
    ))
    .as_error())
}

// ── Key diagnostics / ping ───────────────────────────────────────────────────
//
// Two very different audiences use this:
//  - `ping_key_pool` checks EVERY built-in shared key across all three
//    providers. This is a developer-only tool for spotting dead/throttled
//    keys in the pool everyone's app shares — it must NEVER be reachable from
//    an ordinary user's build, even one who found the devMode secret phrase
//    (see App.vue), because devMode is just a persisted Settings flag, not
//    something tied to a specific machine — anyone can flip it in an
//    otherwise-official build. So gating is done here, at the actual network
//    call, via a SEPARATE compile-time-only flag (`MODLEX_DEV_KEY_PING`) that
//    only ever exists in the developer's own local .env — official/CI builds
//    never set it, so the capability is simply absent from the compiled
//    binary, not just hidden behind a UI toggle a curious user could flip.
//  - `ping_own_key` checks only the single key the CALLER supplies (their own
//    z.ai key from Settings) — safe for anyone, since it only spends their
//    own quota, never the shared pool's.
pub fn dev_key_ping_available() -> bool {
    option_env!("MODLEX_DEV_KEY_PING").is_some()
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PingStatus {
    Ok,
    RateLimited,
    InvalidKey,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPingResult {
    pub provider: String,
    /// First few characters only — never the full key, even in a
    /// developer-only diagnostic (still gets copy-pasted into chat/screenshots).
    pub key_preview: String,
    pub status: PingStatus,
    pub latency_ms: u64,
    pub detail: Option<String>,
    /// Free-text rate-limit/quota info, only populated if the provider's
    /// response actually carried recognizable headers for it — never
    /// fabricated for providers that don't expose this.
    pub quota_hint: Option<String>,
}

fn mask_key(key: &str) -> String {
    let visible: String = key.chars().take(8).collect();
    if key.chars().count() > visible.chars().count() {
        format!("{visible}…")
    } else {
        visible
    }
}

fn ping_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("ping reqwest client")
}

/// Known rate-limit header names across the providers we talk to. Groq
/// documents exactly these; z.ai/Cloudflare aren't known to send them, but
/// scanning for them anyway costs nothing and works for free if either ever
/// adds equivalents — absence just means `quota_hint` stays `None`.
fn extract_quota_hint(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let get = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    let mut parts = Vec::new();
    if let (Some(remaining), Some(limit)) = (
        get("x-ratelimit-remaining-requests"),
        get("x-ratelimit-limit-requests"),
    ) {
        parts.push(format!("запросы: {remaining}/{limit}"));
    }
    if let (Some(remaining), Some(limit)) = (
        get("x-ratelimit-remaining-tokens"),
        get("x-ratelimit-limit-tokens"),
    ) {
        parts.push(format!("токены: {remaining}/{limit}"));
    }
    if let Some(reset) = get("x-ratelimit-reset-tokens") {
        parts.push(format!("сброс токенов через: {reset}"));
    }
    if parts.is_empty() { None } else { Some(parts.join(", ")) }
}

/// Sends one minimal (`max_tokens: 1`) chat completion to `url` with `key`
/// and reports how it went — never returns `Err`, a failed ping IS the
/// result, not an exception.
async fn ping_single_key(provider_label: String, url: String, key: String, model: String) -> KeyPingResult {
    let key_preview = mask_key(&key);
    let start = std::time::Instant::now();
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 1,
    });

    let response = ping_client().post(&url).bearer_auth(&key).json(&body).send().await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(resp) => {
            let status_code = resp.status();
            let quota_hint = extract_quota_hint(resp.headers());
            if status_code.is_success() {
                KeyPingResult {
                    provider: provider_label,
                    key_preview,
                    status: PingStatus::Ok,
                    latency_ms,
                    detail: None,
                    quota_hint,
                }
            } else if status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
                KeyPingResult {
                    provider: provider_label,
                    key_preview,
                    status: PingStatus::RateLimited,
                    latency_ms,
                    detail: Some("HTTP 429".to_string()),
                    quota_hint,
                }
            } else if status_code == reqwest::StatusCode::UNAUTHORIZED
                || status_code == reqwest::StatusCode::FORBIDDEN
            {
                KeyPingResult {
                    provider: provider_label,
                    key_preview,
                    status: PingStatus::InvalidKey,
                    latency_ms,
                    detail: Some(format!("HTTP {status_code}")),
                    quota_hint,
                }
            } else {
                let text: String = resp.text().await.unwrap_or_default().chars().take(200).collect();
                KeyPingResult {
                    provider: provider_label,
                    key_preview,
                    status: PingStatus::Error,
                    latency_ms,
                    detail: Some(format!("HTTP {status_code}: {text}")),
                    quota_hint,
                }
            }
        }
        Err(e) => KeyPingResult {
            provider: provider_label,
            key_preview,
            status: PingStatus::Error,
            latency_ms,
            detail: Some(describe_request_error(&e)),
            quota_hint: None,
        },
    }
}

/// Developer-only: pings EVERY built-in shared key across all three
/// providers concurrently. Refuses to run at all unless
/// `dev_key_ping_available()` — see the module note above for why that's a
/// compile-time flag rather than a runtime/Settings check.
pub async fn ping_key_pool() -> crate::Result<Vec<KeyPingResult>> {
    if !dev_key_ping_available() {
        return Err(crate::ErrorKind::OtherError(
            "Проверка ключей доступна только в сборке разработчика.".to_string(),
        )
        .as_error());
    }

    let mut pings = Vec::new();
    for (i, key) in zai_key_pool().into_iter().enumerate() {
        pings.push(ping_single_key(
            format!("z.ai #{}", i + 1),
            ZAI_CHAT_URL.to_string(),
            key.to_string(),
            ZAI_DEFAULT_MODEL.to_string(),
        ));
    }
    for (i, (account_id, token)) in cloudflare_key_pool().into_iter().enumerate() {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1/chat/completions"
        );
        pings.push(ping_single_key(
            format!("Cloudflare #{}", i + 1),
            url,
            token.to_string(),
            CLOUDFLARE_DEFAULT_MODEL.to_string(),
        ));
    }
    for (i, key) in groq_key_pool().into_iter().enumerate() {
        pings.push(ping_single_key(
            format!("Groq #{}", i + 1),
            GROQ_CHAT_URL.to_string(),
            key.to_string(),
            DEFAULT_GROQ_MODEL.to_string(),
        ));
    }

    Ok(futures::future::join_all(pings).await)
}

/// Anyone can call this — it only ever spends the caller's OWN key/quota,
/// never the shared pool. Always pings against z.ai, since that's the only
/// provider the "own key" Settings field is for.
pub async fn ping_own_key(key: String) -> crate::Result<KeyPingResult> {
    if key.trim().is_empty() {
        return Err(crate::ErrorKind::InputError("Ключ пустой".to_string()).into());
    }
    Ok(ping_single_key(
        "z.ai".to_string(),
        ZAI_CHAT_URL.to_string(),
        key,
        ZAI_DEFAULT_MODEL.to_string(),
    )
    .await)
}

/// Sends a chat completion request (no tools), trying the built-in key pool
/// in order and falling back to `user_key` (from Settings) if every pooled
/// key is rate-limited or none are configured at all. `model` should come
/// from Settings.modlex_ai_model (empty string falls back to the default).
pub async fn chat(
    messages: Vec<AiChatMessage>,
    user_key: Option<String>,
    model: &str,
) -> crate::Result<String> {
    let model = if model.is_empty() {
        DEFAULT_GROQ_MODEL
    } else {
        model
    };
    let message = chat_raw(&messages, user_key, model, None, "auto").await?;
    Ok(message.content.unwrap_or_default())
}

/// Convenience wrapper: chat with the system prompt automatically prepended.
pub async fn chat_with_system_prompt(
    mut messages: Vec<AiChatMessage>,
    user_key: Option<String>,
    model: &str,
) -> crate::Result<String> {
    // Пул непустой => этот запрос МОЖЕТ обслужиться общим ключом (какой
    // конкретно ключ реально ответит — решится только внутри chat(), но для
    // выбора текста промпта достаточно "в принципе может" — иначе пришлось
    // бы дублировать всю логику перебора ключей ещё и здесь).
    let restrict_to_topic = !groq_key_pool().is_empty();
    messages.insert(0, AiChatMessage::system(system_prompt(restrict_to_topic)));
    chat(messages, user_key, model).await
}

// ── Tool-calling ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolTier {
    /// Read-only, scoped to launcher/mod data — executes immediately, no
    /// confirmation. The only tier any tool actually uses in Phase 1.
    Auto,
    /// Data leaves the app to somewhere external (hardware specs, a bug
    /// report to Discord) — always confirmed, even with auto_confirm on.
    AskFirst,
    /// State-changing actions (install/remove/toggle/change-version a mod,
    /// set instance Java/memory, install a Java runtime) — must ask before
    /// running. `agent_loop` stops and returns `PendingConfirmation` instead
    /// of executing; only `agent_resume`, after the user's explicit
    /// confirm/deny, actually calls `execute_tool` for these.
    Confirm,
}

fn tool_tier(name: &str) -> Option<ToolTier> {
    match name {
        "list_instances"
        | "get_instance_details"
        | "list_installed_content"
        | "get_last_crash_report"
        | "get_launcher_log"
        | "search_mods"
        | "get_project_versions"
        | "get_project_details"
        | "list_java_installations"
        | "get_loader_versions"
        | "save_note"
        | "list_notes" => Some(ToolTier::Auto),
        "install_mod_version"
        | "install_mod_versions"
        | "remove_mod"
        | "toggle_mod"
        | "change_mod_version"
        | "set_instance_java"
        | "set_instance_memory"
        | "install_java"
        | "set_instance_loader"
        | "launch_instance"
        | "create_instance"
        | "rename_instance"
        | "update_instance_content" => Some(ToolTier::Confirm),
        "get_system_specs" | "report_bug" => Some(ToolTier::AskFirst),
        _ => None,
    }
}

fn tool_definitions() -> Vec<GroqToolWire> {
    vec![
        groq_tool(
            "list_instances",
            "Список всех установленных инстанций (сборок) лаунчера — id, имя, загрузчик, версия игры.",
            serde_json::json!({"type": "object", "properties": {}}),
        ),
        groq_tool(
            "get_instance_details",
            "Подробности одной инстанции: версия игры, загрузчик, переопределения Java/памяти (null = используется глобальная настройка лаунчера).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"}
                },
                "required": ["instance_id"]
            }),
        ),
        groq_tool(
            "list_installed_content",
            "Список установленных модов/шейдеров/ресурспаков/датапаков в инстанции с версиями.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"}
                },
                "required": ["instance_id"]
            }),
        ),
        groq_tool(
            "get_last_crash_report",
            "Последний краш-репорт КОНКРЕТНОЙ инстанции (крашнулась игра/Minecraft), если он есть, иначе последний лог запуска. Секреты/токены в выводе уже вычищены. Ответ включает created_seconds_ago — отчёт описывает ПРОШЛЫЙ запуск и не обновляется сам по себе, пока пользователь заново не запустит игру. НЕ подходит для жалоб на сам лаунчер (не игру) — для этого get_launcher_log.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"}
                },
                "required": ["instance_id"]
            }),
        ),
        groq_tool(
            "get_launcher_log",
            "Лог самого приложения-лаунчера (не конкретной игры/инстанции) — для жалоб вида 'лаунчер глючит/зависает/не открывается', ошибок интерфейса и т.п. Секреты/токены уже вычищены. Без параметров.",
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        ),
        groq_tool(
            "search_mods",
            "Поиск проектов на Modrinth (моды/шейдеры/ресурспаки/датапаки).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "поисковый запрос"},
                    "project_type": {
                        "type": "string",
                        "enum": ["mod", "resourcepack", "shader", "datapack"],
                        "description": "по умолчанию mod"
                    },
                    "loader": {"type": "string", "description": "например fabric, forge, quilt, neoforge"},
                    "game_version": {"type": "string", "description": "например 1.20.1"}
                },
                "required": ["query"]
            }),
        ),
        groq_tool(
            "get_project_versions",
            "Доступные версии проекта Modrinth, опционально отфильтрованные по загрузчику/версии игры — для подбора совместимой/более старой версии. Каждая версия уже включает поле changelog (реальный список изменений именно ЭТОЙ версии) — используй его, когда спрашивают про историю изменений/что нового/чейнджлог, а не только краткое описание проекта из get_project_details.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {"type": "string"},
                    "loader": {"type": "string"},
                    "game_version": {"type": "string"}
                },
                "required": ["project_id"]
            }),
        ),
        groq_tool(
            "get_project_details",
            "Краткое описание, полный текст страницы проекта (body), категории и поддержка клиент/сервер проекта Modrinth — для рекомендаций и подробных вопросов о моде. За историей изменений конкретных версий смотри get_project_versions (у неё changelog по каждой версии).",
            serde_json::json!({
                "type": "object",
                "properties": {"project_id": {"type": "string"}},
                "required": ["project_id"]
            }),
        ),
        groq_tool(
            "list_java_installations",
            "Список найденных на системе установок Java (версия, архитектура, путь) — для диагностики крашей из-за несовместимой Java.",
            serde_json::json!({"type": "object", "properties": {}}),
        ),
        groq_tool(
            "get_loader_versions",
            "Реальный список доступных версий загрузчика модов (Fabric Loader, Forge и т.п.) для конкретной версии Minecraft — из официального metadata API, а не с Modrinth. Используй перед set_instance_loader, чтобы не придумывать номер версии самому. ВАЖНО: это версия ЗАГРУЗЧИКА, а не мода \"Fabric API\" — они пронумерованы независимо друг от друга.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "loader": {"type": "string", "enum": ["fabric", "forge", "quilt", "neoforge"]},
                    "game_version": {"type": "string", "description": "версия Minecraft, например \"1.20.1\""}
                },
                "required": ["loader", "game_version"]
            }),
        ),
        groq_tool(
            "save_note",
            "Записать заметку для СЕБЯ САМОГО на будущее — история этого диалога не переживает перезапуск лаунчера, а заметки переживают. Пиши сюда, что поставил/удалил/выяснил/решил, если это может понадобиться в следующей сессии (например: \"поставил оптимизационные моды X, Y, пользователь просил без шейдеров\"). Не требует подтверждения.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {"type": "string", "description": "короткая заметка своими словами"},
                    "instance_id": {"type": "string", "description": "если заметка про конкретную инстанцию — её ID; для общих заметок не указывай"}
                },
                "required": ["content"]
            }),
        ),
        groq_tool(
            "list_notes",
            "Прочитать свои прошлые заметки (см. save_note) — общие и, если указан instance_id, ещё и по конкретной инстанции. Вызывай в начале разговора, если похоже, что с этим пользователем/инстанцией уже была работа раньше.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ограничить заметками этой инстанции (плюс общие); без него — только общие"}
                }
            }),
        ),
        groq_tool(
            "get_system_specs",
            "Характеристики железа пользователя (ОС, процессор, объём и свободная оперативная память) — используй, чтобы понять, потянет ли ПК тяжёлую сборку, порекомендовать лимит памяти или объяснить лаги/OutOfMemory. Требует подтверждения пользователя (даже при включённом автоподтверждении обычных действий).",
            serde_json::json!({"type": "object", "properties": {}}),
        ),
        // ── Тир Confirm — требуют подтверждения пользователя, ничего не
        // выполняется без явного клика (см. agent_resume). ────────────────
        groq_tool(
            "install_mod_version",
            "Установить конкретную версию проекта Modrinth в инстанцию — ЗАВИСИМОСТИ резолвятся и ставятся автоматически, отдельно искать/ставить их не нужно. Для НЕСКОЛЬКИХ модов за раз используй install_mod_versions — экономит вызовы и подтверждения. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"},
                    "project_id": {"type": "string", "description": "ID проекта, см. search_mods/get_project_details"},
                    "version_id": {"type": "string", "description": "ID версии, см. get_project_versions"}
                },
                "required": ["instance_id", "project_id", "version_id"]
            }),
        ),
        groq_tool(
            "install_mod_versions",
            "Установить СРАЗУ НЕСКОЛЬКО модов в инстанцию за один вызов и одно подтверждение пользователя — предпочитай этот инструмент install_mod_version, когда нужно поставить больше одного мода (например, собираешь сборку). Зависимости каждого мода резолвятся и ставятся автоматически.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"},
                    "mods": {
                        "type": "array",
                        "description": "Список модов для установки одним разом",
                        "items": {
                            "type": "object",
                            "properties": {
                                "project_id": {"type": "string", "description": "ID проекта, см. search_mods/get_project_details"},
                                "version_id": {"type": "string", "description": "ID версии, см. get_project_versions"}
                            },
                            "required": ["project_id", "version_id"]
                        }
                    }
                },
                "required": ["instance_id", "mods"]
            }),
        ),
        groq_tool(
            "remove_mod",
            "Удалить установленный контент (мод/шейдер/ресурспак/датапак) из инстанции. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "content_path": {"type": "string", "description": "относительный путь файла, см. file_path из list_installed_content"}
                },
                "required": ["instance_id", "content_path"]
            }),
        ),
        groq_tool(
            "toggle_mod",
            "Включить или выключить установленный контент без удаления. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "content_path": {"type": "string", "description": "относительный путь файла, см. file_path из list_installed_content"},
                    "enabled": {"type": "boolean"}
                },
                "required": ["instance_id", "content_path", "enabled"]
            }),
        ),
        groq_tool(
            "change_mod_version",
            "Сменить версию уже установленного мода на другую (например более старую, если новая ломает загрузку). Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "content_path": {"type": "string", "description": "относительный путь файла, см. file_path из list_installed_content"},
                    "version_id": {"type": "string", "description": "ID новой версии, см. get_project_versions"}
                },
                "required": ["instance_id", "content_path", "version_id"]
            }),
        ),
        groq_tool(
            "set_instance_java",
            "Назначить инстанции конкретный путь к Java (переопределение поверх глобальной настройки). Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "java_path": {"type": "string", "description": "путь к исполняемому файлу Java, см. list_java_installations"}
                },
                "required": ["instance_id", "java_path"]
            }),
        ),
        groq_tool(
            "set_instance_memory",
            "Назначить инстанции конкретный лимит памяти (переопределение поверх глобальной настройки). Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "max_mb": {"type": "integer", "description": "максимальная память в мегабайтах"}
                },
                "required": ["instance_id", "max_mb"]
            }),
        ),
        groq_tool(
            "install_java",
            "Скачать и установить в лаунчер указанную мажорную версию Java (например 17, 21), если подходящей нет на системе. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "java_version": {"type": "integer", "description": "мажорная версия Java, например 17"}
                },
                "required": ["java_version"]
            }),
        ),
        groq_tool(
            "set_instance_loader",
            "Сменить загрузчик модов (fabric/forge/quilt/neoforge/vanilla), его версию и/или версию Minecraft у ИНСТАНЦИИ ЦЕЛИКОМ. ВАЖНО: это НЕ мод — не путай с проектом \"Fabric API\" на Modrinth (то отдельный мод-библиотека, устанавливается через install_mod_version). Если краш вызван тем, что версия загрузчика (например Fabric Loader) слишком старая/новая для установленных модов — используй именно этот инструмент, а не install_mod_version. После вызова лаунчер сам скачает нужные файлы загрузчика. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string"},
                    "loader": {"type": "string", "enum": ["vanilla", "fabric", "forge", "quilt", "neoforge"], "description": "новый загрузчик; укажи ТЕКУЩИЙ же загрузчик, если меняешь только версию"},
                    "loader_version": {"type": "string", "description": "версия загрузчика (например \"0.16.13\") — узнать реальный список версий можно через get_loader_versions, не придумывай номер сам. Не нужна для vanilla."},
                    "game_version": {"type": "string", "description": "опционально — сменить заодно и версию Minecraft, например \"1.21.1\""}
                },
                "required": ["instance_id", "loader"]
            }),
        ),
        groq_tool(
            "launch_instance",
            "Запустить игру для инстанции (аккаунтом по умолчанию) — тот же эффект, что кнопка \"Играть\". Не жди, что сразу узнаешь, упала игра или нет — это отдельный процесс; спроси пользователя или (после того как он подтвердит, что попробовал) вызови get_last_crash_report. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"}
                },
                "required": ["instance_id"]
            }),
        ),
        groq_tool(
            "create_instance",
            "Создать новую инстанцию (сборку) — тот же результат, что мастер \"Новая инстанция\" в UI. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "game_version": {"type": "string", "description": "версия Minecraft, например \"1.21.1\""},
                    "loader": {"type": "string", "enum": ["vanilla", "fabric", "forge", "quilt", "neoforge"]},
                    "loader_version": {"type": "string", "description": "версия загрузчика — см. get_loader_versions, не придумывай сам. Не нужна для vanilla."}
                },
                "required": ["name", "game_version", "loader"]
            }),
        ),
        groq_tool(
            "rename_instance",
            "Переименовать существующую инстанцию. Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"},
                    "name": {"type": "string", "description": "новое имя"}
                },
                "required": ["instance_id", "name"]
            }),
        ),
        groq_tool(
            "update_instance_content",
            "Обновить ВСЕ устаревшие моды инстанции до последних совместимых версий разом — не для смены версии загрузчика/игры (для этого set_instance_loader) и не смена модпака как единого целого (такого пути нет). Требует подтверждения пользователя.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "instance_id": {"type": "string", "description": "ID инстанции, см. list_instances"}
                },
                "required": ["instance_id"]
            }),
        ),
        groq_tool(
            "report_bug",
            "Отправить баг-репорт разработчику (в приватный Discord-канал) — только для РЕАЛЬНО подтверждённой проблемы (данные из get_last_crash_report/get_launcher_log, конкретные воспроизводимые шаги), а не по одной лишь просьбе пользователя без деталей. Версия лаунчера/ОС/хвост лога лаунчера подставляются в репорт автоматически кодом — НЕ дублируй их в fields. Обязательно ПОКАЖИ пользователю текстом, что именно отправишь, ДО вызова. Всегда требует подтверждения пользователя, даже в режиме без подтверждений.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "короткий заголовок проблемы"},
                    "description": {"type": "string", "description": "что произошло, как воспроизвести, что ожидалось"},
                    "fields": {
                        "type": "array",
                        "description": "ТОЛЬКО специфичный контекст, которого нет в авто-полях (например конкретная инстанция/мод/версия из разговора) — версию лаунчера/ОС/лог указывать не нужно, это уже есть в отчёте",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "value": {"type": "string"}
                            },
                            "required": ["name", "value"]
                        }
                    }
                },
                "required": ["title", "description"]
            }),
        ),
    ]
}

#[derive(Deserialize, Default)]
struct InstanceIdArgs {
    instance_id: String,
}

#[derive(Deserialize)]
struct SearchModsArgs {
    query: String,
    #[serde(default)]
    project_type: Option<String>,
    #[serde(default)]
    loader: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
}

#[derive(Deserialize)]
struct ProjectVersionsArgs {
    project_id: String,
    #[serde(default)]
    loader: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
}

#[derive(Deserialize)]
struct ProjectIdArgs {
    project_id: String,
}

#[derive(Deserialize)]
struct SaveNoteArgs {
    content: String,
    #[serde(default)]
    instance_id: Option<String>,
}

#[derive(Deserialize)]
struct ListNotesArgs {
    #[serde(default)]
    instance_id: Option<String>,
}

#[derive(Deserialize)]
struct InstallModVersionArgs {
    instance_id: String,
    project_id: String,
    version_id: String,
}

#[derive(Deserialize)]
struct InstallModEntry {
    project_id: String,
    version_id: String,
}

#[derive(Deserialize)]
struct InstallModsArgs {
    instance_id: String,
    mods: Vec<InstallModEntry>,
}

#[derive(Deserialize)]
struct ContentPathArgs {
    instance_id: String,
    content_path: String,
}

#[derive(Deserialize)]
struct ToggleModArgs {
    instance_id: String,
    content_path: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct ChangeModVersionArgs {
    instance_id: String,
    content_path: String,
    version_id: String,
}

#[derive(Deserialize)]
struct SetInstanceJavaArgs {
    instance_id: String,
    java_path: String,
}

#[derive(Deserialize)]
struct SetInstanceMemoryArgs {
    instance_id: String,
    max_mb: u32,
}

#[derive(Deserialize)]
struct InstallJavaArgs {
    java_version: u32,
}

#[derive(Deserialize)]
struct GetLoaderVersionsArgs {
    loader: String,
    game_version: String,
}

#[derive(Deserialize)]
struct SetInstanceLoaderArgs {
    instance_id: String,
    loader: String,
    loader_version: Option<String>,
    game_version: Option<String>,
}

#[derive(Deserialize)]
struct CreateInstanceArgs {
    name: String,
    game_version: String,
    loader: String,
    #[serde(default)]
    loader_version: Option<String>,
}

#[derive(Deserialize)]
struct RenameInstanceArgs {
    instance_id: String,
    name: String,
}

fn parse_mod_loader(loader: &str) -> crate::Result<crate::state::ModLoader> {
    serde_json::from_value(serde_json::Value::String(loader.to_string())).map_err(|_| {
        crate::ErrorKind::InputError(format!(
            "неизвестный загрузчик \"{loader}\" — допустимые значения: vanilla, fabric, forge, quilt, neoforge"
        ))
        .as_error()
    })
}

/// Тир Confirm получает `content_path` от модели — не доверяем ей вслепую.
/// `remove_project`/`toggle_disable_project`/`switch_project_version_with_dependencies`
/// в app-lib делают `base.join(project_path)` БЕЗ проверки, что результат не
/// выходит за пределы инстанции (абсолютный путь полностью заменяет базу —
/// поведение `PathBuf::join`, а `..` может увести наружу директории
/// инстанции) — это существующая брешь в самих этих функциях, отдельная от
/// ИИ-агента, но именно агент — первое место, где путь реально приходит из
/// непроверенного источника (текст, который в теории мог быть подсказан
/// промпт-инъекцией через описание мода). Поэтому проверяем здесь, до
/// вызова: путь должен быть относительным, без `..`, и обязан присутствовать
/// в реальном списке контента этой же инстанции.
/// Чисто синтаксическая часть проверки (без обращения к State/файловой
/// системе) — вынесена отдельно, чтобы её можно было юнит-тестировать без
/// `State::init()` (см. модуль tests в конце файла и примечание о
/// `State::get()` там же).
fn is_safe_relative_content_path(content_path: &str) -> bool {
    let path = Path::new(content_path);
    // `has_root()` (not just `is_absolute()`) — on Windows `/etc/passwd` is
    // NOT `is_absolute()` (no drive letter) but IS rooted, and `PathBuf::join`
    // still treats it as jumping to the current drive's root, escaping the
    // instance directory just as much as a real absolute path would.
    !path.is_absolute()
        && !path.has_root()
        && !path.components().any(|c| c.as_os_str() == "..")
}

async fn ensure_content_path_belongs_to_instance(
    instance_id: &str,
    content_path: &str,
) -> crate::Result<()> {
    if !is_safe_relative_content_path(content_path) {
        return Err(crate::ErrorKind::InputError(format!(
            "недопустимый путь \"{content_path}\""
        ))
        .as_error());
    }

    let items = crate::api::instance::get_content_items(instance_id, None).await?;
    if !items.iter().any(|item| item.file_path == content_path) {
        return Err(crate::ErrorKind::InputError(format!(
            "путь \"{content_path}\" не найден среди установленного контента этой инстанции"
        ))
        .as_error());
    }

    Ok(())
}

fn parse_args<T: for<'de> Deserialize<'de>>(arguments: &str) -> crate::Result<T> {
    let arguments = if arguments.trim().is_empty() {
        "{}"
    } else {
        arguments
    };
    serde_json::from_str(arguments).map_err(|e| {
        crate::ErrorKind::OtherError(format!("не удалось разобрать аргументы: {e}"))
            .as_error()
    })
}

async fn tool_list_instances() -> crate::Result<String> {
    let instances = crate::api::instance::list().await?;
    let summary: Vec<_> = instances
        .into_iter()
        .map(|meta| {
            serde_json::json!({
                "id": meta.instance.id,
                "name": meta.instance.name,
                "game_version": meta.applied_content_set.game_version,
                "loader": meta.applied_content_set.loader.as_str(),
                "loader_version": meta.applied_content_set.loader_version,
            })
        })
        .collect();
    Ok(serde_json::to_string(&summary)?)
}

async fn tool_get_instance_details(arguments: &str) -> crate::Result<String> {
    let args: InstanceIdArgs = parse_args(arguments)?;
    let Some(meta) = crate::api::instance::get(&args.instance_id).await? else {
        return Ok(serde_json::json!({"error": "инстанция не найдена"}).to_string());
    };
    let summary = serde_json::json!({
        "id": meta.instance.id,
        "name": meta.instance.name,
        "game_version": meta.applied_content_set.game_version,
        "loader": meta.applied_content_set.loader.as_str(),
        "loader_version": meta.applied_content_set.loader_version,
        "java_path_override": meta.launch_overrides.java_path,
        "memory_override": meta.launch_overrides.memory,
        "note": "null у override-полей значит используется глобальная настройка лаунчера, не то что Java/память не назначены вовсе",
    });
    Ok(serde_json::to_string(&summary)?)
}

async fn tool_list_installed_content(arguments: &str) -> crate::Result<String> {
    let args: InstanceIdArgs = parse_args(arguments)?;
    let items = crate::api::instance::get_content_items(&args.instance_id, None).await?;
    let summary: Vec<_> = items
        .into_iter()
        .map(|item| {
            serde_json::json!({
                "id": item.id,
                "project_type": item.project_type,
                "enabled": item.enabled,
                "file_name": item.file_name,
                // Относительный путь ("mods/foo.jar") — нужен как content_path
                // для remove_mod/toggle_mod/change_mod_version. Не абсолютный
                // путь на диске, ничего похожего на имя пользователя ОС в нём нет.
                "file_path": item.file_path,
                "project_title": item.project.as_ref().map(|p| p.title.clone()),
                "project_id": item.project.as_ref().map(|p| p.id.clone()),
                "version_number": item.version.as_ref().map(|v| v.version_number.clone()),
                "has_update": item.has_update,
            })
        })
        .collect();
    Ok(serde_json::to_string(&summary)?)
}

/// Log/crash output can be large — keep only the tail, that's where the
/// actual error usually is, so repeated tool calls across a conversation
/// don't blow up the (fully resent-every-turn) context.
const MAX_LOG_CHARS: usize = 6000;

fn truncate_log(output: &str) -> String {
    if output.len() <= MAX_LOG_CHARS {
        return output.to_string();
    }
    let tail_start = output.len() - MAX_LOG_CHARS;
    // Не резать посреди UTF-8-символа.
    let tail_start = (tail_start..output.len())
        .find(|&i| output.is_char_boundary(i))
        .unwrap_or(tail_start);
    format!("...(обрезано, показан конец)...\n{}", &output[tail_start..])
}

async fn tool_get_last_crash_report(arguments: &str) -> crate::Result<String> {
    let args: InstanceIdArgs = parse_args(arguments)?;
    let logs = crate::api::logs::get_logs(&args.instance_id, None).await?;
    // get_logs() уже сортирует по убыванию age (самое свежее — первое).
    let picked = logs
        .iter()
        .find(|l| l.log_type == crate::api::logs::LogType::CrashReport)
        .or_else(|| logs.first());

    let Some(log) = picked else {
        return Ok(serde_json::json!({"note": "логов и краш-репортов не найдено"}).to_string());
    };

    let output = serde_json::to_value(&log.output)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();

    // Живьём наблюдалось (2026-08-29): модель повторно читает этот же самый
    // (старый) краш-репорт ПОСЛЕ того, как сама же внесла исправление в
    // этом разговоре, и делает по нему НОВЫЕ выводы — как будто игру уже
    // перезапустили и она снова упала, хотя на деле файл не менялся, а
    // запуска не было вовсе. Отчёт сам по себе не обновляется, пока
    // пользователь не запустит игру заново — явно отдаём модели, насколько
    // он старый, чтобы она могла это заметить сама (см. также инструкцию в
    // system_prompt).
    let created_secs_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|now| now.as_secs().saturating_sub(log.age))
        .unwrap_or(0);

    let summary = serde_json::json!({
        "log_type": log.log_type,
        "filename": log.filename,
        "created_seconds_ago": created_secs_ago,
        "output": truncate_log(&output),
    });
    Ok(serde_json::to_string(&summary)?)
}

/// Лог самого лаунчера (не игры/инстанции) — то же, что видит человек через
/// "Latest launcher log excerpt" в ручном баг-репорте.
async fn tool_get_launcher_log() -> crate::Result<String> {
    let state = crate::state::State::get().await?;
    let Some((path, tail)) =
        crate::install::diagnostics::latest_launcher_log_tail(&state).await?
    else {
        return Ok(
            serde_json::json!({"note": "лог лаунчера не найден"}).to_string(),
        );
    };
    let censored =
        crate::install::diagnostics::censor_support_text(tail, &state).await?;
    let summary = serde_json::json!({
        "filename": path.file_name().map(|f| f.to_string_lossy().to_string()),
        "output": truncate_log(&censored),
    });
    Ok(serde_json::to_string(&summary)?)
}

/// Максимум результатов поиска, которые уходят модели — экономим контекст.
const SEARCH_RESULT_LIMIT: usize = 8;
/// Ключи, которые оставляем из сырого JSON-хита поиска (Modrinth v3) — сама
/// структура хита нетипизирована на стороне app-lib (`serde_json::Value`),
/// поэтому просто фильтруем известные полезные поля, если они есть.
const SEARCH_HIT_KEYS: &[&str] = &[
    "project_id",
    "id",
    "slug",
    "title",
    "description",
    "categories",
    "client_side",
    "server_side",
    "downloads",
];

fn pick_keys(value: &serde_json::Value, keys: &[&str]) -> serde_json::Value {
    let Some(obj) = value.as_object() else {
        return value.clone();
    };
    let mut picked = serde_json::Map::new();
    for key in keys {
        if let Some(v) = obj.get(*key) {
            picked.insert((*key).to_string(), v.clone());
        }
    }
    serde_json::Value::Object(picked)
}

async fn tool_search_mods(arguments: &str) -> crate::Result<String> {
    let args: SearchModsArgs = parse_args(arguments)?;
    let project_type = args.project_type.as_deref().unwrap_or("mod");

    // Modrinth v3 search переименовал фасеты: project_type -> project_types,
    // versions -> game_versions (проверено живым запросом к api.modrinth.com/v3/search,
    // старые ключи молча дают total_hits:0 без единой ошибки).
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_types:{project_type}")]];
    if let Some(loader) = &args.loader {
        facets.push(vec![format!("categories:{loader}")]);
    }
    if let Some(game_version) = &args.game_version {
        facets.push(vec![format!("game_versions:{game_version}")]);
    }
    let facets_json = serde_json::to_string(&facets).unwrap_or_default();
    let query_suffix = format!(
        "?query={}&facets={}&index=relevance&limit={SEARCH_RESULT_LIMIT}",
        urlencoding::encode(&args.query),
        urlencoding::encode(&facets_json),
    );

    let Some(results) = crate::api::cache::get_search_results_v3(&query_suffix, None).await?
    else {
        return Ok(serde_json::json!({"hits": []}).to_string());
    };

    let hits: Vec<_> = results
        .result
        .hits
        .iter()
        .take(SEARCH_RESULT_LIMIT)
        .map(|hit| pick_keys(hit, SEARCH_HIT_KEYS))
        .collect();

    Ok(serde_json::to_string(&serde_json::json!({ "hits": hits }))?)
}

/// Обрезает по границе символа (не байта) с начала — как truncate_log, но
/// сохраняет НАЧАЛО текста (обычно там суть) вместо конца.
fn truncate_changelog(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => format!("{}...(обрезано)", &text[..byte_idx]),
        None => text.to_string(),
    }
}

async fn tool_get_project_versions(arguments: &str) -> crate::Result<String> {
    let args: ProjectVersionsArgs = parse_args(arguments)?;
    let Some(mut versions) =
        crate::api::cache::get_project_versions(&args.project_id, None).await?
    else {
        return Ok(serde_json::json!({"error": "проект не найден"}).to_string());
    };

    if let Some(loader) = &args.loader {
        versions.retain(|v| v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader)));
    }
    if let Some(game_version) = &args.game_version {
        versions.retain(|v| v.game_versions.iter().any(|gv| gv == game_version));
    }
    versions.truncate(10);

    // get_project_versions() above hits Modrinth's list endpoint with
    // include_changelog=false (it's shared with the UI's fast-loading version
    // picker, which never needs changelog text) — so every entry's
    // `changelog` is always None at this point, regardless of what the real
    // data holds. Re-fetch just these (already-filtered, already ≤10)
    // versions by id via the batch /v2/versions endpoint, which DOES include
    // real changelog text, and use that instead.
    let ids: Vec<&str> = versions.iter().map(|v| v.id.as_str()).collect();
    let changelogs: std::collections::HashMap<String, Option<String>> =
        crate::api::cache::get_version_many(&ids, None)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|v| (v.id, v.changelog))
            .collect();

    // Ченджлог может быть длинным (иногда — весь список коммитов) — модели
    // нужна суть, не полный текст; обрезаем как и логи (см. truncate_log).
    const MAX_CHANGELOG_CHARS: usize = 1500;

    let summary: Vec<_> = versions
        .into_iter()
        .map(|v| {
            let changelog = changelogs.get(&v.id).cloned().flatten();
            serde_json::json!({
                "id": v.id,
                "version_number": v.version_number,
                "version_type": v.version_type,
                "game_versions": v.game_versions,
                "loaders": v.loaders,
                "date_published": v.date_published,
                "changelog": changelog.map(|c| truncate_changelog(&c, MAX_CHANGELOG_CHARS)),
            })
        })
        .collect();

    Ok(serde_json::to_string(&summary)?)
}

async fn tool_get_project_details(arguments: &str) -> crate::Result<String> {
    let args: ProjectIdArgs = parse_args(arguments)?;
    let Some(project) = crate::api::cache::get_project(&args.project_id, None).await? else {
        return Ok(serde_json::json!({"error": "проект не найден"}).to_string());
    };

    // MODLEX 2026-09-08: раньше отдавали только `description` (короткое
    // summary с карточки проекта) — реального полного текста страницы
    // (`body`, тот же markdown, что видит человек на Modrinth) агент вообще
    // не мог увидеть, даже когда его прямо об этом просили. `body` бывает
    // длинным (полноценная страница с разделами/картинками-ссылками) —
    // обрезаем тем же способом, что и changelog у версий.
    const MAX_BODY_CHARS: usize = 3000;

    let summary = serde_json::json!({
        "id": project.id,
        "slug": project.slug,
        "title": project.title,
        "description": project.description,
        "body": truncate_changelog(&project.body, MAX_BODY_CHARS),
        "categories": project.categories,
        "client_side": project.client_side,
        "server_side": project.server_side,
        "downloads": project.downloads,
    });
    Ok(serde_json::to_string(&summary)?)
}

async fn tool_list_java_installations() -> crate::Result<String> {
    let jres = crate::api::jre::find_filtered_jres(None).await?;
    let summary: Vec<_> = jres
        .into_iter()
        .map(|j| {
            serde_json::json!({
                "version": j.version,
                "parsed_version": j.parsed_version,
                "architecture": j.architecture,
                "path": j.path,
            })
        })
        .collect();
    Ok(serde_json::to_string(&summary)?)
}

#[derive(sqlx::FromRow)]
struct AgentNoteRow {
    id: i64,
    instance_id: Option<String>,
    content: String,
    created: i64,
}

/// Заметки агента — что поставил/менял/выяснил, чтобы в СЛЕДУЮЩЕЙ сессии
/// (история чата не переживает перезапуск лаунчера, см. комментарий в
/// ModlexAiAgent.vue) не начинать с нуля и не переспрашивать то, что уже
/// выяснено. Пишет и читает сам агент по своему усмотрению — не показывается
/// пользователю отдельным UI, только через list_notes самого агента.
async fn tool_save_note(arguments: &str) -> crate::Result<String> {
    let args: SaveNoteArgs = parse_args(arguments)?;
    let state = crate::state::State::get().await?;
    let created = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO modlex_ai_notes (instance_id, content, created) VALUES (?, ?, ?)",
    )
    .bind(&args.instance_id)
    .bind(&args.content)
    .bind(created)
    .execute(&state.pool)
    .await?;
    Ok(serde_json::json!({"saved": true}).to_string())
}

/// Максимум заметок за раз — как и с логами (см. MAX_LOG_CHARS), история
/// уходит в LLM целиком на каждый ход, копить туда всё подряд без предела
/// нельзя.
const MAX_NOTES: i64 = 50;

async fn tool_list_notes(arguments: &str) -> crate::Result<String> {
    let args: ListNotesArgs = parse_args(arguments)?;
    let state = crate::state::State::get().await?;

    // Заметки конкретной инстанции + общие (instance_id IS NULL) вместе —
    // общие заметки (например, предпочтения пользователя по модам вообще)
    // релевантны независимо от того, о какой инстанции сейчас разговор.
    let rows: Vec<AgentNoteRow> = if let Some(instance_id) = &args.instance_id {
        sqlx::query_as(
            "SELECT id, instance_id, content, created FROM modlex_ai_notes
             WHERE instance_id = ? OR instance_id IS NULL
             ORDER BY created DESC LIMIT ?",
        )
        .bind(instance_id)
        .bind(MAX_NOTES)
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, instance_id, content, created FROM modlex_ai_notes
             ORDER BY created DESC LIMIT ?",
        )
        .bind(MAX_NOTES)
        .fetch_all(&state.pool)
        .await?
    };

    let notes: Vec<_> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "instance_id": row.instance_id,
                "content": row.content,
                "created": row.created,
            })
        })
        .collect();
    Ok(serde_json::json!({ "notes": notes }).to_string())
}

/// Тир AskFirst (не Auto, не Confirm) — единственный зарегистрированный
/// пример этого тира на сейчас (см. doc-комментарий модуля вверху файла).
/// В отличие от Confirm, ВСЕГДА требует явного согласия пользователя, даже
/// если включён автоподтверждение для обычных действий — характеристики
/// железа это не launcher-состояние, но и не совсем "просто почитать
/// список модов", так что оставляем чуть более консервативным.
async fn tool_get_system_specs() -> crate::Result<String> {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    const BYTES_PER_MIB: u64 = 1024 * 1024;
    let cpu_brand = sys.cpus().first().map(|c| c.brand().trim().to_string());

    Ok(serde_json::json!({
        "os": sysinfo::System::long_os_version().or_else(sysinfo::System::name),
        "cpu_brand": cpu_brand,
        "cpu_cores": sys.cpus().len(),
        "total_memory_mb": sys.total_memory() / BYTES_PER_MIB,
        "available_memory_mb": sys.available_memory() / BYTES_PER_MIB,
    })
    .to_string())
}

// ── Тир Confirm — вызываются ТОЛЬКО из agent_resume, после явного согласия
// пользователя (см. execute_tool ниже — маршрутизация та же самая, тир сам
// по себе ничего не блокирует на этом уровне, блокировка — в agent_loop). ──

/// Ставит один проект В МЕСТЕ С зависимостями — тот же движок, что и обычная
/// кнопка "Установить" в UI (`resolve_install_plan` + `install_resolved_content_plan`
/// в `state::instances::commands`), но вызванный синхронно (не через
/// `install_project_with_dependencies`, который стреляет фоновой `tokio::spawn`
/// и возвращает план ДО завершения самой установки — агенту нужно дождаться
/// реального результата, чтобы честно отчитаться пользователю, а не гадать).
/// Раньше агент ставил только сам мод (`add_project_from_version`, без
/// резолва зависимостей) — отсюда баг "поставил аддон Create, забыл про сам
/// Create": агент даже не подозревал, что у аддона есть обязательная
/// зависимость, раз инструмент её не резолвил.
async fn install_one_with_dependencies(
    instance_id: &str,
    project_id: &str,
    version_id: &str,
) -> crate::Result<serde_json::Value> {
    let state = crate::state::State::get().await?;
    // resolve_install_plan/install_resolved_content_plan сами это не проверяют
    // (проверка живёт в приватной ensure_instance_content_unlocked внутри
    // api::instance::projects, недоступной отсюда) — дублируем ту же самую
    // проверку карантина явно, чтобы агент не мог поставить мод в
    // заблокированную (например, привязанную к неизменяемому модпаку)
    // инстанцию в обход обычного пути установки.
    if crate::state::instances::adapters::sqlite::instance_rows::is_instance_quarantined(
        instance_id,
        &state.pool,
    )
    .await?
    {
        return Err(crate::ErrorKind::OtherError(
            "инстанция заблокирована для изменений (например, из-за проблем с модпаком) — установка недоступна".to_string(),
        )
        .as_error());
    }
    let plan = crate::state::instances::commands::resolve_install_plan(
        instance_id,
        crate::state::instances::commands::InstanceInstallProjectRequest {
            project_id: project_id.to_string(),
            version_id: Some(version_id.to_string()),
            content_type: modrinth_content_management::ContentType::Mod,
            selected: Default::default(),
        },
        &state,
    )
    .await?;

    crate::state::instances::commands::install_resolved_content_plan(
        instance_id,
        &plan,
        &state,
    )
    .await?;

    Ok(serde_json::json!({
        "project_id": plan.primary.project_id,
        "dependencies_installed": plan.dependencies.iter().map(|d| d.project_id.clone()).collect::<Vec<_>>(),
    }))
}

async fn tool_install_mod_version(arguments: &str) -> crate::Result<String> {
    let args: InstallModVersionArgs = parse_args(arguments)?;
    let result = install_one_with_dependencies(
        &args.instance_id,
        &args.project_id,
        &args.version_id,
    )
    .await?;
    crate::event::emit::emit_instance(
        &args.instance_id,
        crate::event::InstancePayloadType::Edited,
    )
    .await?;
    Ok(result.to_string())
}

/// Пачка модов ЗА ОДНО подтверждение вместо одного tool-хопа на мод — раньше
/// "собрать техническую сборку" из 4-5 модов упиралось в лимит хопов ещё на
/// стадии установки (поиск+версии+установка отдельно на каждый мод), не
/// говоря уже про повторный поиск/уточнения. Каждый элемент по-прежнему
/// резолвит СВОИ зависимости независимо — если два мода из пачки делят
/// общую зависимость, она просто не переустановится повторно (движок сам
/// проверяет уже установленные project_id, см. `existing_project_ids` в
/// `resolve_install_plan`).
async fn tool_install_mod_versions(arguments: &str) -> crate::Result<String> {
    let args: InstallModsArgs = parse_args(arguments)?;
    let mut installed = Vec::with_capacity(args.mods.len());
    let mut failed = Vec::new();

    for item in &args.mods {
        match install_one_with_dependencies(
            &args.instance_id,
            &item.project_id,
            &item.version_id,
        )
        .await
        {
            Ok(result) => installed.push(result),
            Err(error) => failed.push(serde_json::json!({
                "project_id": item.project_id,
                "error": error.to_string(),
            })),
        }
    }

    crate::event::emit::emit_instance(
        &args.instance_id,
        crate::event::InstancePayloadType::Edited,
    )
    .await?;

    Ok(serde_json::json!({ "installed": installed, "failed": failed }).to_string())
}

async fn tool_remove_mod(arguments: &str) -> crate::Result<String> {
    let args: ContentPathArgs = parse_args(arguments)?;
    ensure_content_path_belongs_to_instance(&args.instance_id, &args.content_path)
        .await?;
    crate::api::instance::remove_project(&args.instance_id, &args.content_path)
        .await?;
    Ok(serde_json::json!({"removed": args.content_path}).to_string())
}

async fn tool_toggle_mod(arguments: &str) -> crate::Result<String> {
    let args: ToggleModArgs = parse_args(arguments)?;
    ensure_content_path_belongs_to_instance(&args.instance_id, &args.content_path)
        .await?;
    let new_path = crate::api::instance::toggle_disable_project(
        &args.instance_id,
        &args.content_path,
        Some(args.enabled),
    )
    .await?;
    Ok(serde_json::json!({"new_path": new_path, "enabled": args.enabled}).to_string())
}

async fn tool_change_mod_version(arguments: &str) -> crate::Result<String> {
    let args: ChangeModVersionArgs = parse_args(arguments)?;
    ensure_content_path_belongs_to_instance(&args.instance_id, &args.content_path)
        .await?;
    let new_path = crate::api::instance::switch_project_version_with_dependencies(
        &args.instance_id,
        &args.content_path,
        &args.version_id,
    )
    .await?;
    Ok(serde_json::json!({"new_path": new_path}).to_string())
}

async fn tool_set_instance_java(arguments: &str) -> crate::Result<String> {
    let args: SetInstanceJavaArgs = parse_args(arguments)?;
    crate::api::instance::edit(
        &args.instance_id,
        crate::state::EditInstance {
            launch_overrides: Some(crate::state::InstanceLaunchOverridesPatch {
                java_path: Some(Some(args.java_path.clone())),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;
    Ok(serde_json::json!({"java_path": args.java_path}).to_string())
}

async fn tool_set_instance_memory(arguments: &str) -> crate::Result<String> {
    let args: SetInstanceMemoryArgs = parse_args(arguments)?;
    crate::api::instance::edit(
        &args.instance_id,
        crate::state::EditInstance {
            launch_overrides: Some(crate::state::InstanceLaunchOverridesPatch {
                memory: Some(Some(crate::state::MemorySettings {
                    maximum: args.max_mb,
                })),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;
    Ok(serde_json::json!({"max_mb": args.max_mb}).to_string())
}

async fn tool_install_java(arguments: &str) -> crate::Result<String> {
    let args: InstallJavaArgs = parse_args(arguments)?;
    let path =
        crate::api::jre::auto_install_java_with_loading(args.java_version, true)
            .await?;
    Ok(serde_json::json!({"installed_path": path.to_string_lossy()}).to_string())
}

async fn tool_get_loader_versions(arguments: &str) -> crate::Result<String> {
    let args: GetLoaderVersionsArgs = parse_args(arguments)?;
    let loader = parse_mod_loader(&args.loader)?;
    let manifest =
        crate::api::metadata::get_loader_versions(loader.as_meta_str()).await?;

    let versions: Vec<_> = manifest
        .game_versions
        .into_iter()
        .find(|v| v.id == args.game_version)
        .map(|v| {
            v.loaders
                .into_iter()
                .take(15)
                .map(|l| serde_json::json!({"version": l.id, "stable": l.stable}))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(serde_json::json!({
        "loader": args.loader,
        "game_version": args.game_version,
        "versions": versions,
    })
    .to_string())
}

/// Меняет загрузчик/версию загрузчика/версию игры у ИНСТАНЦИИ (не мод!) —
/// та же операция, что делает кнопка "Change version" в настройках
/// инстанции: правит запись через `instance::edit`, затем сразу запускает
/// `install_existing_instance`, чтобы реально скачать нужные файлы
/// загрузчика (иначе правка осталась бы только в БД до следующего запуска).
async fn tool_set_instance_loader(arguments: &str) -> crate::Result<String> {
    let args: SetInstanceLoaderArgs = parse_args(arguments)?;
    let loader = parse_mod_loader(&args.loader)?;

    crate::api::instance::edit(
        &args.instance_id,
        crate::state::EditInstance {
            content_set_patch: Some(crate::state::AppliedContentSetPatch {
                loader: Some(loader),
                loader_version: args.loader_version.clone().map(Some),
                game_version: args.game_version.clone(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;

    crate::install::install_existing_instance(args.instance_id.clone(), false)
        .await?;

    Ok(serde_json::json!({
        "loader": args.loader,
        "loader_version": args.loader_version,
        "game_version": args.game_version,
    })
    .to_string())
}

/// Запускает игру — тот же путь, что кнопка "Play" (аккаунт по умолчанию,
/// без quick-play). Не ждёт завершения игры (это отдельный долгоживущий
/// процесс) — только подтверждает, что запуск начался. Если игра упадёт,
/// пользователь уже знает как спросить про причину (get_last_crash_report),
/// а не ждать, что агент сам это заметит "в реальном времени" — постоянный
/// стриминг лога в LLM был бы дорого и почти всегда бесполезно (шум).
async fn tool_launch_instance(arguments: &str) -> crate::Result<String> {
    let args: InstanceIdArgs = parse_args(arguments)?;
    let process = crate::api::instance::run(
        &args.instance_id,
        crate::api::instance::QuickPlayType::None,
    )
    .await?;

    Ok(serde_json::json!({
        "launched": true,
        "instance_name": process.instance_name,
        "process_id": process.uuid,
    })
    .to_string())
}

/// Создаёт новую инстанцию — тот же путь, что мастер "Новая инстанция" в UI.
/// Возвращает instance_id сразу (создаётся синхронно), но фактическая
/// докачка файлов загрузчика идёт дальше уже фоновой install-джобой — как и
/// в обычном UI, ждать полного завершения тут не нужно.
async fn tool_create_instance(arguments: &str) -> crate::Result<String> {
    let args: CreateInstanceArgs = parse_args(arguments)?;
    let loader = parse_mod_loader(&args.loader)?;

    let snapshot = crate::install::create_instance(
        args.name,
        args.game_version,
        loader,
        args.loader_version,
        None,
        None,
        crate::state::InstanceLink::Unmanaged,
    )
    .await?;

    Ok(serde_json::json!({
        "instance_id": snapshot.instance_id,
        "job_id": snapshot.job_id,
    })
    .to_string())
}

async fn tool_rename_instance(arguments: &str) -> crate::Result<String> {
    let args: RenameInstanceArgs = parse_args(arguments)?;
    crate::api::instance::edit(
        &args.instance_id,
        crate::state::EditInstance {
            name: Some(args.name.clone()),
            ..Default::default()
        },
    )
    .await?;
    Ok(serde_json::json!({"instance_id": args.instance_id, "name": args.name}).to_string())
}

/// Обновляет ВСЕ устаревшие моды инстанции до последних совместимых версий
/// разом — тот же путь, что кнопка "Обновить всё" в списке контента. Не
/// путать с обновлением МОДПАКА как единого целого (сменой версии сборки) —
/// такого отдельного пути на сейчас нет, здесь именно про отдельные моды.
async fn tool_update_instance_content(arguments: &str) -> crate::Result<String> {
    let args: InstanceIdArgs = parse_args(arguments)?;
    let updated = crate::api::instance::update_all_projects(&args.instance_id).await?;
    Ok(serde_json::json!({ "updated": updated }).to_string())
}

/// Выполняет один вызов инструмента и ВСЕГДА возвращает `String` — успех или
/// человекочитаемую ошибку — а не `Result`, чтобы одна кривая попытка модели
/// (неизвестный тул, битый JSON аргументов, инстанция не найдена и т.п.) не
/// обрывала весь ход, а модель могла увидеть текст ошибки и подстроиться.
async fn execute_tool(name: &str, arguments: &str) -> String {
    let result: crate::Result<String> = match name {
        "list_instances" => tool_list_instances().await,
        "get_instance_details" => tool_get_instance_details(arguments).await,
        "list_installed_content" => tool_list_installed_content(arguments).await,
        "get_last_crash_report" => tool_get_last_crash_report(arguments).await,
        "get_launcher_log" => tool_get_launcher_log().await,
        "search_mods" => tool_search_mods(arguments).await,
        "get_project_versions" => tool_get_project_versions(arguments).await,
        "get_project_details" => tool_get_project_details(arguments).await,
        "list_java_installations" => tool_list_java_installations().await,
        "install_mod_version" => tool_install_mod_version(arguments).await,
        "install_mod_versions" => tool_install_mod_versions(arguments).await,
        "remove_mod" => tool_remove_mod(arguments).await,
        "toggle_mod" => tool_toggle_mod(arguments).await,
        "change_mod_version" => tool_change_mod_version(arguments).await,
        "get_loader_versions" => tool_get_loader_versions(arguments).await,
        "save_note" => tool_save_note(arguments).await,
        "list_notes" => tool_list_notes(arguments).await,
        "get_system_specs" => tool_get_system_specs().await,
        "launch_instance" => tool_launch_instance(arguments).await,
        "create_instance" => tool_create_instance(arguments).await,
        "rename_instance" => tool_rename_instance(arguments).await,
        "update_instance_content" => tool_update_instance_content(arguments).await,
        "report_bug" => tool_report_bug(arguments).await,
        "set_instance_loader" => tool_set_instance_loader(arguments).await,
        "set_instance_java" => tool_set_instance_java(arguments).await,
        "set_instance_memory" => tool_set_instance_memory(arguments).await,
        "install_java" => tool_install_java(arguments).await,
        "" | " " => Err(crate::ErrorKind::OtherError(
            "получен вызов инструмента с пустым именем — судя по всему, \
             произошла ошибка генерации. Повтори вызов, указав ТОЧНОЕ имя \
             нужного инструмента из списка доступных."
                .to_string(),
        )
        .as_error()),
        other => Err(crate::ErrorKind::OtherError(format!(
            "неизвестный инструмент \"{other}\" — такого нет в списке \
             доступных, проверь точное написание имени."
        ))
        .as_error()),
    };

    match result {
        Ok(s) => s,
        Err(e) => format!("Ошибка: {e}"),
    }
}

/// Итог одного хода агента. `Reply` — обычный текстовый ответ (после любого
/// числа автоматических вызовов read-only инструментов). `PendingConfirmation`
/// — модель вызвала инструмент тира `Confirm`/`AskFirst`: `messages` уже
/// содержит сообщение ассистента с этим tool_calls (и стаб-результатами для
/// любых ДРУГИХ вызовов из той же пачки — см. `agent_loop`), но НЕ результат
/// для `pending_call`. Фронт обязан показать подтверждение и вызвать
/// `agent_resume` с решением пользователя, прежде чем что-либо продолжится.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AgentStepOutcome {
    Reply {
        messages: Vec<AiChatMessage>,
        text: String,
    },
    PendingConfirmation {
        messages: Vec<AiChatMessage>,
        pending_call: AiToolCall,
    },
}

fn resolve_model_and_tools(model: &str) -> (&str, Option<Vec<GroqToolWire>>) {
    let model = if model.is_empty() {
        DEFAULT_GROQ_MODEL
    } else {
        model
    };
    let tools = if model_supports_custom_tools(model) {
        Some(tool_definitions())
    } else {
        None
    };
    (model, tools)
}

/// Общее ядро для `agent_step` и `agent_resume`: прогоняет `messages` через
/// Groq, автоматически выполняя любые Tier::Auto вызовы инструментов (до
/// `MAX_TOOL_HOPS` раз подряд), пока не получит обычный текстовый ответ или
/// не наткнётся на вызов, требующий подтверждения. Предполагает, что
/// системный промпт уже вставлен вызывающей стороной.
async fn agent_loop(
    mut messages: Vec<AiChatMessage>,
    user_key: Option<String>,
    model: &str,
    tools: Option<&[GroqToolWire]>,
    provider: &str,
    auto_confirm: bool,
    max_hops: usize,
    hop_tx: Option<tokio::sync::mpsc::UnboundedSender<AiChatMessage>>,
) -> crate::Result<AgentStepOutcome> {
    // Живьём наблюдалось (2026-08-29, на нескольких провайдерах): модель
    // периодически присылает вызов с пустым/нераспознанным именем — не
    // обязательно подряд раз за разом, иногда вперемешку с реально
    // удачными вызовами (отсюда счётчик суммарный за весь ход, а не только
    // подряд идущих — со "строго подряд" порог никогда не срабатывал на
    // практике). Обрываем раньше, а не перемалываем весь MAX_TOOL_HOPS
    // вхолостую в основном на одних и тех же ошибках.
    const MAX_TOTAL_UNRECOGNIZED: usize = 3;
    let mut total_unrecognized: usize = 0;

    // Стримим КАЖДОЕ сообщение (ответ ассистента и результат каждого
    // инструмента) фронту сразу же, а не только в конце всего хода —
    // раньше был просто статус-текст ("Поиск модов..."), но пользователь
    // всё равно видел итог только одним пакетом в самом конце: и текст
    // ассистента, и все пузыри инструментов разом. Ошибку отправки
    // игнорируем осознанно: получателя может не быть (фронт не
    // слушает/окно закрыто) — это не должно ронять сам запрос.
    let emit = |msg: &AiChatMessage| {
        if let Some(tx) = &hop_tx {
            let _ = tx.send(msg.clone());
        }
    };

    for _ in 0..max_hops {
        let assistant_message =
            chat_raw(&messages, user_key.clone(), model, tools, provider).await?;

        let calls = assistant_message
            .tool_calls
            .clone()
            .filter(|c| !c.is_empty());

        let Some(calls) = calls else {
            let text = assistant_message.content.clone().unwrap_or_default();
            // 2026-09-08: реально наблюдалось живьём (нестабильная сеть до
            // z.ai в этот вечер) — провайдер иногда отдаёт финальное
            // сообщение БЕЗ tool_calls и БЕЗ реального текста (пустая
            // строка/только пробелы). Раньше это тихо уходило на фронт как
            // AgentStepOutcome::Reply с text="" — а пустой текст фронт
            // (renderItems, ModlexAiAgent.vue) вообще не рисует пузырём
            // (там `if (visibleContent)`), так что пользователь видел
            // "ничего не произошло" вместо любой обратной связи. Раз
            // сообщение всё равно пустое — по сути то же самое, что "не
            // получилось", так что даём тот же явный текст, что и для
            // исчерпания max_hops/повторяющихся нераспознанных вызовов,
            // вместо тихого пустого ответа.
            if text.trim().is_empty() {
                let text = "Модель ответила пустым сообщением — похоже, сбой на \
                             стороне провайдера. Попробуй повторить запрос ещё раз \
                             или сменить провайдера в настройках агента."
                    .to_string();
                let final_message = AiChatMessage {
                    role: "assistant".to_string(),
                    content: Some(text.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                };
                emit(&final_message);
                messages.push(final_message);
                return Ok(AgentStepOutcome::Reply { messages, text });
            }
            emit(&assistant_message);
            messages.push(assistant_message);
            return Ok(AgentStepOutcome::Reply { messages, text });
        };

        emit(&assistant_message);
        messages.push(assistant_message);

        for (index, call) in calls.iter().enumerate() {
            match tool_tier(&call.name) {
                Some(ToolTier::Auto) => {
                    let result = execute_tool(&call.name, &call.arguments).await;
                    let tool_message = AiChatMessage::tool_result(call.id.clone(), result);
                    emit(&tool_message);
                    messages.push(tool_message);
                }
                Some(ToolTier::Confirm) if auto_confirm => {
                    // Пользователь явно включил "не спрашивать" в настройках
                    // (с одноразовым предупреждением при включении, см.
                    // фронт) — Confirm-tier выполняется сразу, как Auto.
                    // AskFirst сюда НЕ попадает и это осознанно: там дело не
                    // в риске ошибки, а в приватности (данные уходят
                    // наружу, во внешний API) — такое подтверждают всегда,
                    // авто-режим на это не распространяется.
                    let result = execute_tool(&call.name, &call.arguments).await;
                    let tool_message = AiChatMessage::tool_result(call.id.clone(), result);
                    emit(&tool_message);
                    messages.push(tool_message);
                }
                Some(ToolTier::AskFirst) | Some(ToolTier::Confirm) => {
                    // На случай, если модель в одном ходе вызвала несколько
                    // инструментов и НЕ первый из них требует подтверждения —
                    // всем ОСТАЛЬНЫМ вызовам из этой же пачки нужен хоть
                    // какой-то tool-результат, иначе сообщение ассистента
                    // останется с tool_calls без ответа и следующий же запрос
                    // к Groq упадёт с ошибкой формата (сам gpt-oss на Groq
                    // параллельных вызовов не делает, но код не должен молча
                    // ломаться, если это когда-нибудь изменится).
                    for skipped in &calls[index + 1..] {
                        let skipped_message = AiChatMessage::tool_result(
                            skipped.id.clone(),
                            "Пропущено: ожидает подтверждения другого действия \
                             в этом же ходе."
                                .to_string(),
                        );
                        emit(&skipped_message);
                        messages.push(skipped_message);
                    }
                    return Ok(AgentStepOutcome::PendingConfirmation {
                        messages,
                        pending_call: call.clone(),
                    });
                }
                None => {
                    total_unrecognized += 1;
                    let result = if call.name.trim().is_empty() {
                        "Ошибка: получен вызов инструмента с пустым именем — судя \
                         по всему, произошла ошибка генерации. Повтори вызов, \
                         указав ТОЧНОЕ имя нужного инструмента из списка \
                         доступных."
                            .to_string()
                    } else {
                        format!(
                            "Ошибка: неизвестный инструмент \"{}\" — такого нет \
                             в списке доступных, проверь точное написание имени.",
                            call.name
                        )
                    };
                    let tool_message = AiChatMessage::tool_result(call.id.clone(), result);
                    emit(&tool_message);
                    messages.push(tool_message);

                    if total_unrecognized >= MAX_TOTAL_UNRECOGNIZED {
                        let text = "Не получается сформировать корректный вызов \
                                     инструмента — похоже, сейчас проблема на \
                                     стороне модели/провайдера. Попробуй сменить \
                                     провайдера в настройках агента или повторить \
                                     запрос позже."
                            .to_string();
                        let final_message = AiChatMessage {
                            role: "assistant".to_string(),
                            content: Some(text.clone()),
                            tool_calls: None,
                            tool_call_id: None,
                        };
                        emit(&final_message);
                        messages.push(final_message);
                        return Ok(AgentStepOutcome::Reply { messages, text });
                    }
                }
            }
        }
    }

    let text = "Не смог разобраться за разумное число шагов — попробуй \
                переформулировать вопрос или уточнить детали."
        .to_string();
    let final_message = AiChatMessage {
        role: "assistant".to_string(),
        content: Some(text.clone()),
        tool_calls: None,
        tool_call_id: None,
    };
    emit(&final_message);
    messages.push(final_message);
    Ok(AgentStepOutcome::Reply { messages, text })
}

/// Один ход диалога с агентом. Стейта на сервере нет: `messages` — это ВСЯ
/// история, включая уже случившиеся tool_calls/tool-результаты, и именно её
/// (расширенную) фронт обязан прислать обратно на следующий ход.
pub async fn agent_step(
    mut messages: Vec<AiChatMessage>,
    user_key: Option<String>,
    model: &str,
    provider: &str,
    auto_confirm: bool,
    max_hops: u32,
    hop_tx: Option<tokio::sync::mpsc::UnboundedSender<AiChatMessage>>,
) -> crate::Result<AgentStepOutcome> {
    let (model, tools) = resolve_model_and_tools(model);

    if messages.first().map(|m| m.role.as_str()) != Some("system") {
        // Тему ограничиваем только когда запрос МОЖЕТ уйти в общий пул —
        // если провайдер форсирован на что-то конкретное, тот же критерий
        // (пул этого провайдера непуст) решает, не только Groq.
        let restrict_to_topic = wants_provider(provider, "groq") && !groq_key_pool().is_empty()
            || wants_provider(provider, "zai") && !zai_key_pool().is_empty()
            || wants_provider(provider, "cloudflare") && !cloudflare_key_pool().is_empty();
        messages.insert(0, AiChatMessage::system(system_prompt(restrict_to_topic)));
    }

    agent_loop(
        messages,
        user_key,
        model,
        tools.as_deref(),
        provider,
        auto_confirm,
        resolve_max_hops(max_hops),
        hop_tx,
    )
    .await
}

/// Продолжает диалог после того, как пользователь подтвердил или отклонил
/// вызов из `AgentStepOutcome::PendingConfirmation`. `messages` должен быть
/// тем же массивом, что пришёл в этом `PendingConfirmation` (последнее
/// сообщение в нём — ответ ассистента с ожидающим `pending_call`, без своего
/// tool-результата) — фронт просто пересылает его как есть, не меняя.
pub async fn agent_resume(
    mut messages: Vec<AiChatMessage>,
    pending_call: AiToolCall,
    approved: bool,
    user_key: Option<String>,
    model: &str,
    provider: &str,
    auto_confirm: bool,
    max_hops: u32,
    hop_tx: Option<tokio::sync::mpsc::UnboundedSender<AiChatMessage>>,
) -> crate::Result<AgentStepOutcome> {
    let (model, tools) = resolve_model_and_tools(model);

    let result = if approved {
        execute_tool(&pending_call.name, &pending_call.arguments).await
    } else {
        "Пользователь отклонил выполнение этого действия.".to_string()
    };
    messages.push(AiChatMessage::tool_result(pending_call.id, result));

    agent_loop(
        messages,
        user_key,
        model,
        tools.as_deref(),
        provider,
        auto_confirm,
        resolve_max_hops(max_hops),
        hop_tx,
    )
    .await
}

// ── Список моделей ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelInfo {
    pub id: String,
}

#[derive(Deserialize)]
struct GroqModelsResponse {
    data: Vec<GroqModelEntry>,
}

#[derive(Deserialize)]
struct GroqModelEntry {
    id: String,
    active: bool,
}

/// Fetches the live list of chat-capable models currently served by Groq,
/// using the first available pool key. Groq's lineup changes over time —
/// this is the actual source of truth for what to show in a model picker,
/// not a hardcoded list. No personal-key fallback here (2026-09-06) — the
/// "own key" Settings field is a z.ai credential now, not a Groq one, so it
/// can't authenticate against Groq's `/models` endpoint.
pub async fn list_models() -> crate::Result<Vec<AiModelInfo>> {
    let pool = groq_key_pool();
    let key = pool
        .first()
        .copied()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Нет доступного ключа Groq — не могу получить список моделей."
                    .to_string(),
            )
            .as_error()
        })?;

    let client = groq_client();
    let response = client
        .get(GROQ_MODELS_URL)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(crate::ErrorKind::OtherError(format!(
            "Groq models API error {status}: {text}"
        ))
        .as_error());
    }

    let parsed: GroqModelsResponse = response
        .json()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let mut models: Vec<AiModelInfo> = parsed
        .data
        .into_iter()
        .filter(|m| {
            m.active
                && !NON_CHAT_MODEL_MARKERS
                    .iter()
                    .any(|marker| m.id.to_lowercase().contains(marker))
        })
        .map(|m| AiModelInfo { id: m.id })
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(models)
}

// ── Баг-репорты ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct DiscordWebhookPayload {
    embeds: Vec<DiscordEmbed>,
}

#[derive(Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<DiscordEmbedField>,
}

#[derive(Serialize)]
struct DiscordEmbedField {
    name: String,
    value: String,
    inline: bool,
}

/// Отправляет баг-репорт в приватный Discord-канал через вебхук.
/// Вызывающая сторона (агент) обязана сначала показать пользователю, что
/// именно уйдёт в репорт, и отправлять только после явного подтверждения —
/// эта функция сама по себе никаких подтверждений не спрашивает.
pub async fn send_bug_report(
    title: &str,
    description: &str,
    fields: Vec<(String, String)>,
) -> crate::Result<()> {
    let Some(webhook_url) = option_env!("MODLEX_BUG_REPORT_WEBHOOK") else {
        return Err(crate::ErrorKind::OtherError(
            "Отправка баг-репортов не настроена в этой сборке.".to_string(),
        )
        .as_error());
    };

    let payload = DiscordWebhookPayload {
        embeds: vec![DiscordEmbed {
            title: title.to_string(),
            description: description.to_string(),
            color: 0xF16436, // тот же оранжевый, что и у CF-акцентов в UI
            fields: fields
                .into_iter()
                .map(|(name, value)| DiscordEmbedField {
                    name,
                    // Discord режет значения полей на 1024 символа
                    value: value.chars().take(1000).collect(),
                    inline: false,
                })
                .collect(),
        }],
    };

    let client = groq_client();
    let response = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(crate::ErrorKind::OtherError(format!(
            "Discord webhook error {status}: {text}"
        ))
        .as_error());
    }

    Ok(())
}

#[derive(Deserialize)]
struct BugReportField {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct ReportBugArgs {
    title: String,
    description: String,
    #[serde(default)]
    fields: Vec<BugReportField>,
}

/// send_bug_report() существовала с самого начала (см. её doc-комментарий —
/// она изначально задумывалась как вызываемая агентом), но ни разу не была
/// подключена ни к какому UI, ни к самому агенту как инструмент — по факту
/// мёртвый код. Подключаем как есть.
/// Технические поля (версия/ОС/лог) подставляются здесь кодом, а не моделью —
/// модель отвечает только за title/description/свои доп. поля (см. память
/// project_modlex_curseforge про причину: раньше это целиком зависело от
/// того, не забудет ли модель их включить).
async fn tool_report_bug(arguments: &str) -> crate::Result<String> {
    let args: ReportBugArgs = parse_args(arguments)?;

    let mut fields = Vec::new();

    let mut environment = String::new();
    crate::install::diagnostics::write_environment_details(&mut environment);
    fields.push(("Окружение".to_string(), environment.trim().to_string()));

    if let Ok(state) = crate::state::State::get().await {
        if let Ok(Some((_, log_tail))) =
            crate::install::diagnostics::latest_launcher_log_tail(&state).await
        {
            if let Ok(censored) =
                crate::install::diagnostics::censor_support_text(log_tail, &state).await
            {
                let tail: String = censored.chars().rev().take(1000).collect();
                let tail: String = tail.chars().rev().collect();
                fields.push(("Лог лаунчера (конец)".to_string(), tail));
            }
        }
    }

    fields.extend(args.fields.into_iter().map(|f| (f.name, f.value)));

    send_bug_report(&args.title, &args.description, fields).await?;
    Ok(serde_json::json!({"sent": true}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Временный ручной тест обвязки поверх Groq — не мок, реальный сетевой
    // вызов реальным ключом из .env, чтобы убедиться, что структуры
    // действительно совпадают с тем, что реально отдаёт API (а не просто с
    // тем, что предположено по OpenAI-совместимости). Прогнать вручную:
    // `cargo test modlex_ai -- --nocapture`.
    #[tokio::test]
    async fn live_chat_smoke_test() {
        let reply = chat_with_system_prompt(
            vec![AiChatMessage {
                role: "user".to_string(),
                content: Some("Ответь ровно одним словом: Работаю".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            None,
            DEFAULT_GROQ_MODEL,
        )
        .await
        .expect("chat() should succeed with a valid pooled key");

        println!("Groq replied: {reply}");
        assert!(!reply.trim().is_empty());
    }

    #[tokio::test]
    async fn live_list_models_smoke_test() {
        let models = list_models()
            .await
            .expect("list_models() should succeed with a valid pooled key");
        println!("Groq chat-capable models: {models:?}");
        assert!(!models.is_empty());
        assert!(
            models.iter().any(|m| m.id == DEFAULT_GROQ_MODEL),
            "default model should be present in the live list"
        );
    }

    #[tokio::test]
    async fn live_off_topic_refusal_smoke_test() {
        let reply = chat_with_system_prompt(
            vec![AiChatMessage {
                role: "user".to_string(),
                content: Some(
                    "Привет! Кстати, какая сегодня погода и что ты думаешь о политике?"
                        .to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
            }],
            None,
            DEFAULT_GROQ_MODEL,
        )
        .await
        .expect("chat() should succeed");

        println!("Off-topic reply: {reply}");
        // Не строгая проверка (это LLM, не regex-автомат) — просто убеждаемся,
        // что ответ короткий (не начал рассуждать о погоде/политике всерьёз)
        // и содержит намёк на то, что помогает только с лаунчером.
        assert!(reply.chars().count() < 400, "refusal should be short");
    }

    /// Живой тест ПРОТОКОЛА вызова инструментов (реальный Groq, не мок) —
    /// проверяет, что запрос с `tools` реально принимается Groq и что ответ с
    /// `tool_calls` реально парсится в `AiToolCall` так, как задумано.
    ///
    /// Намеренно НЕ через `agent_step()` и НЕ выполняет сам инструмент:
    /// `execute_tool()` для любого из 8 тир-0 инструментов трогает
    /// `State::get()`, а `State` в голом `cargo test` никогда не
    /// инициализирован (полный бутстрап — `State::init()` — происходит
    /// только при реальном запуске приложения). `State::get()` в этом
    /// случае не падает с ошибкой, а крутится в вечном цикле ожидания (см.
    /// `State::get()` в `state/mod.rs`) — так уже случайно завис один прогон
    /// этого тестового файла на час, когда тест писался через `agent_step`.
    /// Реальное выполнение инструментов проверяется только руками в
    /// запущенном лаунчере (Tauri уже вызывает `State::init()` при старте).
    #[tokio::test]
    async fn live_groq_tool_call_protocol_smoke_test() {
        let messages = vec![AiChatMessage {
            role: "user".to_string(),
            content: Some(
                "Сколько у меня сейчас установлено инстанций лаунчера? \
                 Обязательно используй инструмент list_instances, не угадывай."
                    .to_string(),
            ),
            tool_calls: None,
            tool_call_id: None,
        }];
        let tools = tool_definitions();

        let assistant_message = chat_raw(&messages, None, DEFAULT_GROQ_MODEL, Some(&tools), "groq")
            .await
            .expect("chat_raw() should succeed with a valid pooled key");

        println!("Assistant message: {assistant_message:?}");
        let calls = assistant_message
            .tool_calls
            .expect("model should have requested a tool call for this question");
        assert!(
            calls.iter().any(|c| c.name == "list_instances"),
            "expected a list_instances call, got: {calls:?}"
        );
    }

    /// Живой тест резервного провайдера (z.ai, реальный ключ из .env, не
    /// мок) — бьёт напрямую в `try_openai_compatible_call` с ZAI_CHAT_URL,
    /// В ОБХОД пула Groq (иначе пришлось бы реально исчерпать Groq, чтобы
    /// дойти до этой ветки). Подтверждает, что наши структуры `GroqRequest`/
    /// `AiChatMessage` реально парсят настоящий ответ z.ai (не только что
    /// curl вручную получал 200 — тут именно наш код разбирает JSON), и что
    /// z.ai's `tools`/`tool_calls` формат совместим с тем же самым разбором,
    /// что и у Groq (лишнее поле `reasoning_content` в ответе z.ai должно
    /// молча игнорироваться serde, не ломая парсинг).
    #[tokio::test]
    async fn live_zai_fallback_protocol_smoke_test() {
        let pool = zai_key_pool();
        let Some(key) = pool.first() else {
            eprintln!("ZAI_API_KEY_1 не задан в .env — пропускаю тест");
            return;
        };

        let messages = vec![AiChatMessage {
            role: "user".to_string(),
            content: Some("Ответь ровно одним словом: Работаю".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let message = try_openai_compatible_call(ZAI_CHAT_URL, key, ZAI_DEFAULT_MODEL, &messages, None)
            .await
            .expect("try_openai_compatible_call() should succeed against z.ai")
            .expect("z.ai should not rate-limit a single test request");

        println!("z.ai replied: {message:?}");
        assert!(
            message.content.as_deref().is_some_and(|c| !c.trim().is_empty()),
            "expected non-empty content, got: {message:?}"
        );
    }

    /// То же самое, но для третьего резервного провайдера — Cloudflare
    /// Workers AI (реальный account_id/token из .env, не мок). Отдельно от
    /// zai-теста, потому что тут проверяется ещё и правильная сборка URL с
    /// подставленным account_id.
    #[tokio::test]
    async fn live_cloudflare_fallback_protocol_smoke_test() {
        let pool = cloudflare_key_pool();
        let Some((account_id, token)) = pool.first() else {
            eprintln!("CLOUDFLARE_ACCOUNT_ID_1/CLOUDFLARE_API_TOKEN_1 не заданы в .env — пропускаю тест");
            return;
        };

        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1/chat/completions"
        );
        let messages = vec![AiChatMessage {
            role: "user".to_string(),
            content: Some("Ответь ровно одним словом: Работаю".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        let message = try_openai_compatible_call(&url, token, CLOUDFLARE_DEFAULT_MODEL, &messages, None)
            .await
            .expect("try_openai_compatible_call() should succeed against Cloudflare Workers AI")
            .expect("Cloudflare should not rate-limit a single test request");

        println!("Cloudflare replied: {message:?}");
        assert!(
            message.content.as_deref().is_some_and(|c| !c.trim().is_empty()),
            "expected non-empty content, got: {message:?}"
        );
    }

    // Чистая синтаксическая проверка, без сети и без State — можно гонять в
    // любом окружении. Настоящая (State-зависимая) часть проверки —
    // "путь реально принадлежит этой инстанции" — тестируется только руками
    // в запущенном лаунчере, по той же причине, что и остальные тир-Confirm
    // инструменты (см. комментарий у live_groq_tool_call_protocol_smoke_test).
    #[test]
    fn rejects_unsafe_content_paths() {
        assert!(!is_safe_relative_content_path("C:\\Users\\me\\mods\\foo.jar"));
        assert!(!is_safe_relative_content_path("/etc/passwd"));
        assert!(!is_safe_relative_content_path("../../outside.jar"));
        assert!(!is_safe_relative_content_path("mods/../../../escape.jar"));
        assert!(is_safe_relative_content_path("mods/foo.jar"));
        assert!(is_safe_relative_content_path("shaderpacks/bar.zip"));
    }

    /// Живой тест протокола для тира Confirm (реальный Groq, не мок) — та же
    /// схема и то же ограничение, что у live_groq_tool_call_protocol_smoke_test
    /// выше: проверяем только разбор ответа Groq (tool_calls реально
    /// распознан), НЕ выполняем сам инструмент (execute_tool для toggle_mod
    /// тоже трогает State::get() через ensure_content_path_belongs_to_instance).
    #[tokio::test]
    async fn live_groq_confirm_tier_tool_call_protocol_smoke_test() {
        let messages = vec![AiChatMessage {
            role: "user".to_string(),
            content: Some(
                "Выключи (disable) мод с относительным путём mods/example.jar \
                 в инстанции с id test-instance. Обязательно используй \
                 инструмент toggle_mod, не угадывай и не спрашивай уточнений."
                    .to_string(),
            ),
            tool_calls: None,
            tool_call_id: None,
        }];
        let tools = tool_definitions();

        let assistant_message = chat_raw(&messages, None, DEFAULT_GROQ_MODEL, Some(&tools), "groq")
            .await
            .expect("chat_raw() should succeed with a valid pooled key");

        println!("Assistant message: {assistant_message:?}");
        let calls = assistant_message
            .tool_calls
            .expect("model should have requested a tool call for this question");
        let call = calls
            .iter()
            .find(|c| c.name == "toggle_mod")
            .unwrap_or_else(|| panic!("expected a toggle_mod call, got: {calls:?}"));

        assert_eq!(tool_tier(&call.name), Some(ToolTier::Confirm));
        let args: serde_json::Value = serde_json::from_str(&call.arguments)
            .expect("toggle_mod arguments should be valid JSON");
        assert_eq!(args["instance_id"], "test-instance");
        assert_eq!(args["content_path"], "mods/example.jar");
        assert_eq!(args["enabled"], false);
    }
}
