-- Find regions whose average ticket is above the global average.

CREATE TABLE IF NOT EXISTS orders (
    id          INTEGER PRIMARY KEY,
    region      TEXT NOT NULL,
    amount      REAL NOT NULL,
    placed_on   TEXT NOT NULL
);

SELECT
    region,
    COUNT(*)        AS order_count,
    SUM(amount)     AS total
FROM orders
WHERE placed_on >= '2026-01-01'
GROUP BY region
HAVING AVG(amount) > (
    SELECT AVG(amount) FROM orders
)
ORDER BY total DESC;