<?php
// PHP — class wrapping PDO, select users active in last 30 days.

declare(strict_types=1);

final class UserRepository
{
    public function __construct(private PDO $db) {}

    /** @return list<array{id:int,email:string,name:string}> */
    public function activeSince(DateTimeImmutable $cutoff): array
    {
        $sql = 'SELECT id, email, name FROM users WHERE last_seen >= :cutoff';
        $stmt = $this->db->prepare($sql);
        $stmt->execute(['cutoff' => $cutoff->format('Y-m-d')]);

        return array_map(
            static fn(array $r): array => [
                'id'    => (int) $r['id'],
                'email' => (string) $r['email'],
                'name'  => (string) $r['name'],
            ],
            $stmt->fetchAll(PDO::FETCH_ASSOC)
        );
    }
}

$cutoff = (new DateTimeImmutable())->sub(new DateInterval('P30D'));
$repo = new UserRepository($pdo);
foreach ($repo->activeSince($cutoff) as $u) {
    echo "{$u['name']} <{$u['email']}>\n";
}