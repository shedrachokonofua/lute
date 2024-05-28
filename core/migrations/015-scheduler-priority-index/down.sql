DROP INDEX idx_scheduler_jobs_name_priority_next_execution_id;
CREATE INDEX idx_scheduler_jobs_name_next_execution_priority_id
  ON scheduler_jobs (name, next_execution, priority, id);