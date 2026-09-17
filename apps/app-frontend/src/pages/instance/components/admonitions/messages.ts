import { defineMessages } from '@modrinth/ui'

export const instanceAdmonitionsMessages = defineMessages({
	sharedInstanceChangesHeader: {
		id: 'app.instance.admonitions.shared-instance.changes-header',
		defaultMessage: "Your changes haven't been shared yet",
	},
	sharedInstanceChangesBody: {
		id: 'app.instance.admonitions.shared-instance.changes-body',
		defaultMessage: "Your local instance is ahead of the users you've shared it with.",
	},
	sharedInstancePublishButton: {
		id: 'app.instance.admonitions.shared-instance.publish-button',
		defaultMessage: 'Push update',
	},
	sharedInstancePublishingButton: {
		id: 'app.instance.admonitions.shared-instance.publishing-button',
		defaultMessage: 'Pushing...',
	},
	sharedInstanceReviewingButton: {
		id: 'app.instance.admonitions.shared-instance.reviewing-button',
		defaultMessage: 'Reviewing...',
	},
	sharedInstanceReviewHeader: {
		id: 'app.instance.admonitions.shared-instance.review-header',
		defaultMessage: 'Review changes',
	},
	sharedInstanceReviewAdmonitionHeader: {
		id: 'app.instance.admonitions.shared-instance.review-admonition-header',
		defaultMessage: 'Push update',
	},
	sharedInstanceReviewDescription: {
		id: 'app.instance.admonitions.shared-instance.review-description',
		defaultMessage:
			'Review the content changes that will be shared with everyone using this instance.',
	},
	sharedInstanceAddedLabel: {
		id: 'app.instance.admonitions.shared-instance.added-label',
		defaultMessage: 'Added',
	},
	sharedInstanceRemovedLabel: {
		id: 'app.instance.admonitions.shared-instance.removed-label',
		defaultMessage: 'Removed',
	},
	sharedInstanceWrongAccountHeader: {
		id: 'app.instance.shared-instance-wrong-account.warning-header',
		defaultMessage: 'You are using the wrong Modrinth account',
	},
	sharedInstanceSignedOutHeader: {
		id: 'app.instance.shared-instance-wrong-account.signed-out-header',
		defaultMessage: 'You need to sign in to Modrinth',
	},
	sharedInstanceWrongAccountSignInAs: {
		id: 'app.instance.shared-instance-wrong-account.sign-in-as-label',
		defaultMessage: 'Sign in as',
	},
	sharedInstanceWrongAccountUserBody: {
		id: 'app.instance.shared-instance-wrong-account.user-admonition-body-v2',
		defaultMessage: 'to receive updates for this shared instance.',
	},
	sharedInstanceWrongAccountOwnerBody: {
		id: 'app.instance.shared-instance-wrong-account.owner-admonition-body-v2',
		defaultMessage: "to manage this shared instance. You won't be able to push updates to users.",
	},
	sharedInstanceWrongAccountFallbackUsername: {
		id: 'app.instance.shared-instance-wrong-account.fallback-username',
		defaultMessage: 'the linked account',
	},
	offlineMultiplayerQuirkHeader: {
		id: 'app.instance.admonitions.offline-multiplayer-quirk.header',
		defaultMessage: 'Multiplayer may be disabled on launch',
	},
	offlineMultiplayerQuirkBody: {
		id: 'app.instance.admonitions.offline-multiplayer-quirk.body',
		defaultMessage:
			"Minecraft 1.16.5 is known to disable the Multiplayer button for offline accounts if the internet is available while it starts up. If this happens, try disabling your internet connection right before launching, then turning it back on afterward — it can stay on for the rest of the session.",
	},
})
