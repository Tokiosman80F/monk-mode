-- A task's time block: when it starts, in minutes after the day starts at 06:00.
ALTER TABLE task ADD COLUMN start_min INTEGER;
