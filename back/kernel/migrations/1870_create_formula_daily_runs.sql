CREATE TABLE formula_daily_runs (
  id TINYINT UNSIGNED NOT NULL,
  local_date DATE NOT NULL,
  ran_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  PRIMARY KEY (id),
  CONSTRAINT formula_daily_runs_single_row CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
