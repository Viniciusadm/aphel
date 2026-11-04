UPDATE runtime_permissions rp
JOIN plugins_installed p ON rp.owner = CONCAT('plugin:', p.plugin_key)
JOIN JSON_TABLE(p.manifest, '$.permissions[*]' COLUMNS (
 permission_key VARCHAR(100) PATH '$.key',
 label TEXT PATH '$.label' NULL ON EMPTY,
 description TEXT PATH '$.description' NULL ON EMPTY
)) declared ON declared.permission_key = rp.permission
SET rp.label = declared.label, rp.description = declared.description;
