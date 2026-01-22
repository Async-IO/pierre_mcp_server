// ABOUTME: Astro configuration for Pierre website landing page
// ABOUTME: Configures Tailwind for the marketing site, docs served via mdBook at /documentation/

import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';

export default defineConfig({
  site: 'https://pierre.async-io.org',
  integrations: [
    tailwind({
      applyBaseStyles: false,
    }),
  ],
  build: {
    assets: 'assets',
  },
});
