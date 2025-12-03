// ABOUTME: Content collection configuration for Starlight docs
// ABOUTME: Defines the docs collection schema for type-safe markdown processing

import { defineCollection } from 'astro:content';
import { docsSchema } from '@astrojs/starlight/schema';

export const collections = {
  docs: defineCollection({ schema: docsSchema() }),
};
