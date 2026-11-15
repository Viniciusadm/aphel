UPDATE installed_modules
SET settings = JSON_SET(
  COALESCE(settings, JSON_OBJECT()),
  '$.amount_mode',
  (SELECT amount_mode FROM line_item_settings WHERE id = 1)
)
WHERE module_key = 'line_items'
  AND EXISTS (SELECT 1 FROM line_item_settings WHERE id = 1);

DROP TABLE line_item_settings;
