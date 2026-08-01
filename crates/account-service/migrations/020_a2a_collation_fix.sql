-- 全库 collation 统一修复（MySQL 1267: Illegal mix of collations）。
--
-- 背景：009/010 等迁移显式使用 utf8mb4_unicode_ci，而 001/002/016 等
-- 建表未声明 COLLATE，继承服务器默认 collation（utf8mb4_general_ci）。
-- 与 unicode_ci 表 JOIN / `=` 比较时触发：
--   1267 (HY000): Illegal mix of collations (utf8mb4_unicode_ci,IMPLICIT)
--                 and (utf8mb4_general_ci,IMPLICIT) for operation '='
--
-- 统一为 utf8mb4_unicode_ci（与显式声明的基础表一致），消除 JOIN 冲突。
-- 幂等：重复执行 CONVERT TO CHARACTER SET 无害。

ALTER TABLE users
  CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

ALTER TABLE organizations
  CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

ALTER TABLE linked_devices
  CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

ALTER TABLE a2a_agent_presence
  CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

ALTER TABLE a2a_handoff_tasks
  CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;