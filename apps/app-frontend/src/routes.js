import { createRouter, createWebHistory } from 'vue-router'

import * as Pages from '@/pages'
import * as Hosting from '@/pages/hosting/manage'
import * as Instance from '@/pages/instance'
import * as Library from '@/pages/library'
import * as Project from '@/pages/project'
import * as CurseForge from '@/pages/curseforge'

/**
 * Configures application routing. Add page to pages/index and then add to route table here.
 */
export default new createRouter({
	history: createWebHistory(),
	routes: [
		{
			path: '/',
			name: 'Home',
			component: Pages.Index,
			meta: {
				breadcrumb: [{ name: 'Home' }],
			},
		},
		{
			path: '/worlds',
			name: 'Worlds',
			component: Pages.Worlds,
			meta: {
				breadcrumb: [{ name: 'Worlds' }],
			},
		},
		{
			path: '/hosting/manage/',
			name: 'Servers',
			component: Pages.Servers,
			meta: {
				breadcrumb: [{ name: 'Servers' }],
			},
		},
		{
			path: '/hosting/manage/:id',
			name: 'ServerManage',
			component: Hosting.Index,
			children: [
				{
					path: '',
					name: 'ServerManageOverview',
					component: Hosting.Overview,
					meta: {
						breadcrumb: [{ name: '?Server' }],
					},
				},
				{
					path: 'content',
					name: 'ServerManageContent',
					component: Hosting.Content,
					meta: {
						breadcrumb: [{ name: '?Server' }],
					},
				},
				{
					path: 'files',
					name: 'ServerManageFiles',
					component: Hosting.Files,
					meta: {
						breadcrumb: [{ name: '?Server' }],
					},
				},
				{
					path: 'backups',
					name: 'ServerManageBackups',
					component: Hosting.Backups,
					meta: {
						breadcrumb: [{ name: '?Server' }],
					},
				},
				{
					path: 'access',
					name: 'ServerManageAccess',
					component: Hosting.Access,
					meta: {
						breadcrumb: [{ name: '?Server' }],
					},
				},
			],
		},
		{
			path: '/browse/:projectType',
			name: 'Discover content',
			component: Pages.Browse,
			meta: {
				useContext: true,
				breadcrumb: [{ name: '?BrowseTitle' }],
			},
		},
		{
			path: '/skins',
			name: 'Skin selector',
			component: Pages.Skins,
			meta: {
				breadcrumb: [{ name: 'Skin selector' }],
			},
		},
		{
			path: '/library',
			name: 'Library',
			component: Library.Index,
			meta: {
				breadcrumb: [{ name: 'Library' }],
			},
			children: [
				{
					path: '',
					name: 'Overview',
					component: Library.Overview,
				},
				{
					path: 'downloaded',
					name: 'Downloaded',
					component: Library.Downloaded,
				},
				{
					path: 'modpacks',
					name: 'Modpacks',
					component: Library.Modpacks,
				},
				{
					path: 'servers',
					name: 'LibraryServers',
					component: Library.Servers,
				},
				{
					path: 'custom',
					name: 'Custom',
					component: Library.Custom,
				},
			],
		},
		{
			path: '/:projectType(mod|plugin|datapack|resourcepack|shader|modpack)/:id/:rest(.*)*',
			redirect: (to) => {
				const rest = to.params.rest ? `/${[].concat(to.params.rest).join('/')}` : ''
				return `/project/${to.params.id}${rest}${to.hash}`
			},
		},
		{
			path: '/project/:id',
			name: 'Project',
			component: Project.Index,
			props: true,
			children: [
				{
					path: '',
					name: 'Description',
					component: Project.Description,
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?Project' }],
					},
				},
				{
					path: 'versions',
					name: 'Versions',
					component: Project.Versions,
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?Project', link: '/project/{id}/' }, { name: 'Versions' }],
					},
				},
				{
					path: 'version/:version',
					name: 'Version',
					component: Project.Version,
					props: true,
					meta: {
						useContext: true,
						breadcrumb: [
							{ name: '?Project', link: '/project/{id}/' },
							{ name: 'Versions', link: '/project/{id}/versions' },
							{ name: '?Version' },
						],
					},
				},
				{
					path: 'gallery',
					name: 'Gallery',
					component: Project.Gallery,
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?Project', link: '/project/{id}/' }, { name: 'Gallery' }],
					},
				},
			],
		},
		{
			path: '/curseforge/:id',
			name: 'CurseForgeProject',
			component: CurseForge.Index,
			props: true,
			children: [
				{
					path: '',
					name: 'CurseForgeDescription',
					component: CurseForge.Description,
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?CurseForgeProject' }],
					},
				},
				{
					path: 'versions',
					name: 'CurseForgeVersions',
					component: CurseForge.Versions,
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?CurseForgeProject', link: '/curseforge/{id}/' }, { name: 'Versions' }],
					},
				},
				{
					path: 'version/:version',
					name: 'CurseForgeVersion',
					component: CurseForge.Version,
					props: true,
					meta: {
						useContext: true,
						breadcrumb: [
							{ name: '?CurseForgeProject', link: '/curseforge/{id}/' },
							{ name: 'Versions', link: '/curseforge/{id}/versions' },
							{ name: '?Version' },
						],
					},
				},
				{
					path: 'gallery',
					name: 'CurseForgeGallery',
					component: CurseForge.Gallery,
					meta: {
						useContext: true,
						breadcrumb: [{ name: '?CurseForgeProject', link: '/curseforge/{id}/' }, { name: 'Gallery' }],
					},
				},
			],
		},
		{
			path: '/instance/:id',
			name: 'Instance',
			component: Instance.Index,
			props: true,
			children: [
				{
					path: 'worlds',
					name: 'InstanceWorlds',
					component: Instance.Worlds,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Worlds' }],
					},
				},
				{
					path: '',
					name: 'Mods',
					component: Instance.Mods,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Content' }],
					},
				},
				{
					path: 'projects/:type',
					name: 'ModsFilter',
					component: Instance.Mods,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Content' }],
					},
				},
				{
					path: 'files',
					name: 'Files',
					component: Instance.Files,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Files' }],
					},
				},
				{
					path: 'logs',
					name: 'Logs',
					component: Instance.Logs,
					meta: {
						useRootContext: true,
						breadcrumb: [{ name: '?Instance', link: '/instance/{id}/' }, { name: 'Logs' }],
					},
				},
			],
		},
	],
	linkActiveClass: 'router-link-active',
	linkExactActiveClass: 'router-link-exact-active',
	scrollBehavior(to, from) {
		if (to.path === from.path) return
		document.querySelector('.app-viewport')?.scrollTo(0, 0)
		return {
			el: '.app-viewport',
			top: 0,
		}
	},
})
