DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS customers;
DROP TABLE IF EXISTS refunds;
CREATE TABLE customers(id INTEGER PRIMARY KEY, name TEXT, region TEXT, industry TEXT);
CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INTEGER, amount REAL, status TEXT, created_at TEXT);
CREATE TABLE refunds(id INTEGER PRIMARY KEY, order_id INTEGER, amount REAL, reason TEXT, created_at TEXT);
INSERT INTO customers VALUES
  (1, '上海云启', '华东', 'SaaS'),
  (2, '广州辰星', '华南', '零售'),
  (3, '北京北辰', '华北', '制造');
INSERT INTO orders VALUES
  (1, 1, 12000, 'paid', '2026-06-01'),
  (2, 1, 18000, 'paid', '2026-06-04'),
  (3, 2, 9000, 'paid', '2026-06-02'),
  (4, 3, 7000, 'paid', '2026-06-03'),
  (5, 3, 6500, 'paid', '2026-06-05');
INSERT INTO refunds VALUES
  (1, 4, 1200, '质量问题', '2026-06-06'),
  (2, 5, 800, '响应慢', '2026-06-07');
