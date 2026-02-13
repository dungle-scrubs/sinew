import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

const site = "https://sinew.dev";

export default defineConfig({
	site,
	integrations: [
		starlight({
			title: "Sinew",
			description:
				"A macOS menu bar replacement built in Rust. Notch-aware layouts, modular widgets, hot-reload config.",
			logo: {
				src: "./src/assets/logo.jpg",
				alt: "Sinew",
			},
			favicon: "/favicon.png",
			head: [
				{
					tag: "link",
					attrs: {
						rel: "apple-touch-icon",
						href: "/apple-touch-icon.png",
					},
				},
				{
					tag: "meta",
					attrs: { property: "og:image", content: `${site}/og-image.png` },
				},
				{
					tag: "meta",
					attrs: { property: "og:image:width", content: "1200" },
				},
				{
					tag: "meta",
					attrs: { property: "og:image:height", content: "630" },
				},
				{
					tag: "meta",
					attrs: {
						property: "og:image:alt",
						content: "Sinew — macOS menu bar replacement",
					},
				},
				{
					tag: "meta",
					attrs: { name: "twitter:card", content: "summary_large_image" },
				},
				{
					tag: "meta",
					attrs: { name: "twitter:image", content: `${site}/og-image.png` },
				},
			],
			social: [
				{
					icon: "github",
					label: "GitHub",
					href: "https://github.com/dungle-scrubs/sinew",
				},
			],
			customCss: ["./src/styles/custom.css"],
			sidebar: [
				{
					label: "Getting Started",
					autogenerate: { directory: "getting-started" },
				},
				{
					label: "Modules",
					autogenerate: { directory: "modules" },
				},
				{
					label: "Guides",
					autogenerate: { directory: "guides" },
				},
				{
					label: "Reference",
					autogenerate: { directory: "reference" },
				},
			],
		}),
	],
});
