-- ABOUTME: Adds activity linkage to shared insights for coach-mediated sharing
-- ABOUTME: Links insights to their source activities and marks coach-generated content

-- Add source activity linkage to shared insights
ALTER TABLE shared_insights ADD COLUMN source_activity_id TEXT;

-- Mark whether insight was coach-generated vs manual
ALTER TABLE shared_insights ADD COLUMN coach_generated INTEGER DEFAULT 0;

-- Index for looking up insights by activity
CREATE INDEX idx_shared_insights_activity ON shared_insights(source_activity_id);

-- Index for filtering coach-generated insights
CREATE INDEX idx_shared_insights_coach_generated ON shared_insights(coach_generated) WHERE coach_generated = 1;
