// Singly linked list with insert + reverse.

#include <stdio.h>
#include <stdlib.h>

struct Node {
    int value;
    struct Node *next;
};

struct Node *push_front(struct Node *head, int v) {
    struct Node *n = malloc(sizeof(struct Node));
    if (!n) return head;
    n->value = v;
    n->next = head;
    return n;
}

struct Node *reverse(struct Node *head) {
    struct Node *prev = NULL;
    struct Node *cur = head;
    while (cur) {
        struct Node *nx = cur->next;
        cur->next = prev;
        prev = cur;
        cur = nx;
    }
    return prev;
}

void print_list(const struct Node *head) {
    for (; head; head = head->next) {
        printf("%d ", head->value);
    }
    printf("\n");
}

int main(void) {
    struct Node *list = NULL;
    for (int i = 0; i < 6; i++) {
        list = push_front(list, i * 10);
    }
    print_list(list);
    list = reverse(list);
    print_list(list);
    return 0;
}