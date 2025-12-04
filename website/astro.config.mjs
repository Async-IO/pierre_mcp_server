// ABOUTME: Astro configuration for Pierre website with Starlight docs
// ABOUTME: Configures Tailwind, Starlight navigation, i18n, and Pierre branding

import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwind from '@astrojs/tailwind';

export default defineConfig({
  site: 'https://pierre.async-io.org',
  integrations: [
    starlight({
      title: 'Pierre Fitness Intelligence',
      logo: {
        src: './src/assets/pierre-logo.svg',
        alt: 'Pierre Fitness Intelligence',
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/Async-IO/pierre_mcp_server' },
      ],
      customCss: [
        './src/styles/pierre-theme.css',
      ],
      // Internationalization with root locale for English
      defaultLocale: 'root',
      locales: {
        root: {
          label: 'English',
          lang: 'en',
        },
        fr: {
          label: 'Francais',
          lang: 'fr',
        },
      },
      sidebar: [
        {
          label: 'Getting Started',
          translations: {
            fr: 'Demarrer',
          },
          items: [
            { label: 'Overview', slug: 'docs/getting-started', translations: { fr: 'Vue d\'ensemble' } },
            { label: 'Installation', slug: 'docs/installation-guides/install-mcp-client', translations: { fr: 'Installation' } },
            { label: 'Configuration', slug: 'docs/configuration', translations: { fr: 'Configuration' } },
          ],
        },
        {
          label: 'Core Concepts',
          translations: {
            fr: 'Concepts Cles',
          },
          items: [
            { label: 'Architecture', slug: 'docs/architecture', translations: { fr: 'Architecture' } },
            { label: 'Protocols', slug: 'docs/protocols', translations: { fr: 'Protocoles' } },
            { label: 'Authentication', slug: 'docs/authentication', translations: { fr: 'Authentification' } },
          ],
        },
        {
          label: 'Intelligence',
          translations: {
            fr: 'Intelligence',
          },
          items: [
            { label: 'Methodology', slug: 'docs/intelligence-methodology', translations: { fr: 'Methodologie' } },
            { label: 'Nutrition', slug: 'docs/nutrition-methodology', translations: { fr: 'Nutrition' } },
          ],
        },
        {
          label: 'References',
          translations: {
            fr: 'References',
          },
          items: [
            { label: 'Tools Reference', slug: 'docs/tools-reference', translations: { fr: 'Reference des Outils' } },
            { label: 'OAuth2 Server', slug: 'docs/oauth2-server', translations: { fr: 'Serveur OAuth2' } },
            { label: 'OAuth Client', slug: 'docs/oauth-client', translations: { fr: 'Client OAuth' } },
            { label: 'Provider Registration', slug: 'docs/provider-registration-guide', translations: { fr: 'Enregistrement des Fournisseurs' } },
          ],
        },
        {
          label: 'Development',
          translations: {
            fr: 'Developpement',
          },
          items: [
            { label: 'Development Guide', slug: 'docs/development', translations: { fr: 'Guide de Developpement' } },
            { label: 'Build', slug: 'docs/build', translations: { fr: 'Compilation' } },
            { label: 'Testing', slug: 'docs/testing', translations: { fr: 'Tests' } },
            { label: 'Testing Strategy', slug: 'docs/testing-strategy', translations: { fr: 'Strategie de Test' } },
            { label: 'CI/CD', slug: 'docs/ci-cd', translations: { fr: 'CI/CD' } },
            { label: 'Contributing', slug: 'docs/contributing', translations: { fr: 'Contribuer' } },
          ],
        },
      ],
    }),
    tailwind({
      applyBaseStyles: false,
    }),
  ],
});
