<?php
// PHP — procedural style, array_map pipeline over user list.

declare(strict_types=1);

function active_since(array $users, DateTimeImmutable $cutoff): array
{
    $active = array_filter(
        $users,
        static fn(array $u): bool =>
            isset($u['last_seen']) &&
            new DateTimeImmutable($u['last_seen']) >= $cutoff
    );

    return array_map(
        static fn(array $u): string => sprintf('%s <%s>', $u['name'], $u['email']),
        array_values($active)
    );
}

$users = [
    ['name' => 'Ada',  'email' => 'ada@example.com',  'last_seen' => '2026-07-30'],
    ['name' => 'Linus','email' => 'linus@example.com','last_seen' => '2026-06-01'],
];

$cutoff = (new DateTimeImmutable())->sub(new DateInterval('P30D'));
foreach (active_since($users, $cutoff) as $line) {
    echo $line . "\n";
}