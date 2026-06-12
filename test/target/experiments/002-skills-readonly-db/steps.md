1. 用 `readonly_seed.sql` 创建 SQLite 数据库。
2. 将数据库文件 chmod 为 readonly。
3. 执行固定 SELECT 查询并导出 CSV。
4. 故意执行 UPDATE。
5. 校验 SELECT 成功、UPDATE 失败且错误包含 readonly。
