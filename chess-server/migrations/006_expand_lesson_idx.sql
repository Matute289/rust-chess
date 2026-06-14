-- Expand lesson_idx range from 0-4 to 0-99999 to support up to 100k lessons
ALTER TABLE lesson_progress DROP CONSTRAINT lesson_progress_lesson_idx_check;
ALTER TABLE lesson_progress ADD CONSTRAINT lesson_progress_lesson_idx_check
    CHECK (lesson_idx >= 0 AND lesson_idx < 100000);
