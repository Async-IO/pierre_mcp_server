-- ABOUTME: Migration to add sample_prompts column to coaches table.
-- ABOUTME: Stores JSON array of sample prompts for quick-start suggestions.

-- Add sample_prompts column to coaches table
-- This allows coaches to have associated sample prompts that users can
-- quickly select to start a conversation with the coach.
ALTER TABLE coaches ADD COLUMN sample_prompts TEXT;
