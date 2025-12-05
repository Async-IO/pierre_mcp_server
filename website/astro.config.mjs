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
            { label: 'Overview', slug: 'getting-started', translations: { fr: 'Vue d\'ensemble' } },
            { label: 'Installation', slug: 'installation-guides/install-mcp-client', translations: { fr: 'Installation' } },
            { label: 'Configuration', slug: 'configuration', translations: { fr: 'Configuration' } },
          ],
        },
        {
          label: 'Core Concepts',
          translations: {
            fr: 'Concepts Cles',
          },
          items: [
            { label: 'Architecture', slug: 'architecture', translations: { fr: 'Architecture' } },
            { label: 'Protocols', slug: 'protocols', translations: { fr: 'Protocoles' } },
            { label: 'Authentication', slug: 'authentication', translations: { fr: 'Authentification' } },
          ],
        },
        {
          label: 'Intelligence',
          translations: {
            fr: 'Intelligence',
          },
          items: [
            { label: 'Methodology', slug: 'intelligence-methodology', translations: { fr: 'Methodologie' } },
            { label: 'Nutrition', slug: 'nutrition-methodology', translations: { fr: 'Nutrition' } },
          ],
        },
        {
          label: 'References',
          translations: {
            fr: 'References',
          },
          items: [
            { label: 'Tools Reference', slug: 'tools-reference', translations: { fr: 'Reference des Outils' } },
            { label: 'OAuth2 Server', slug: 'oauth2-server', translations: { fr: 'Serveur OAuth2' } },
            { label: 'OAuth Client', slug: 'oauth-client', translations: { fr: 'Client OAuth' } },
            { label: 'Provider Registration', slug: 'provider-registration-guide', translations: { fr: 'Enregistrement des Fournisseurs' } },
          ],
        },
        {
          label: 'Development',
          translations: {
            fr: 'Developpement',
          },
          items: [
            { label: 'Development Guide', slug: 'development', translations: { fr: 'Guide de Developpement' } },
            { label: 'Build', slug: 'build', translations: { fr: 'Compilation' } },
            { label: 'Testing', slug: 'testing', translations: { fr: 'Tests' } },
            { label: 'Testing Strategy', slug: 'testing-strategy', translations: { fr: 'Strategie de Test' } },
            { label: 'CI/CD', slug: 'ci-cd', translations: { fr: 'CI/CD' } },
            { label: 'Contributing', slug: 'contributing', translations: { fr: 'Contribuer' } },
          ],
        },
      ],
    }),
    tailwind({
      applyBaseStyles: false,
    }),
  ],
});
