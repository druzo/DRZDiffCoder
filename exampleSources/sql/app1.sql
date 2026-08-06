-- Sales totals by region for orders placed in Q1 2026.

CREATE TABLE IF NOT EXISTS orders (
    id          INTEGER PRIMARY KEY,
    region      TEXT NOT NULL,
    amount      REAL NOT NULL,
    placed_on   TEXT NOT NULL
);

SELECT
    o.region           AS region,
    COUNT(*)           AS order_count,
    SUM(o.amount)      AS total,
    AVG(o.amount)      AS avg_ticket
FROM orders AS o
JOIN customers AS c ON c.id = o.customer_id
WHERE o.placed_on BETWEEN '2026-01-01' AND '2026-03-31'
  AND c.active = 1
GROUP BY o.region
HAVING COUNT(*) >= 5
ORDER BY total DESC;