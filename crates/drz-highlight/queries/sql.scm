; SQL syntax highlighting using tree-sitter-sequel 0.3.11 grammar node names

; Comments and strings
(comment) @comment
(keyword_string) @string

; Numbers are not separate named nodes in sequel grammar

; Constants
(keyword_null) @constant
(keyword_true) @constant
(keyword_false) @constant

; SQL keywords (named nodes keyword_*)
(keyword_select) @keyword
(keyword_from) @keyword
(keyword_where) @keyword
(keyword_join) @keyword
(keyword_on) @keyword
(keyword_group) @keyword
(keyword_by) @keyword
(keyword_order) @keyword
(keyword_as) @keyword
(keyword_and) @keyword
(keyword_or) @keyword
(keyword_not) @keyword
(keyword_insert) @keyword
(keyword_into) @keyword
(keyword_values) @keyword
(keyword_update) @keyword
(keyword_set) @keyword
(keyword_delete) @keyword
(keyword_create) @keyword
(keyword_table) @keyword
(keyword_drop) @keyword
(keyword_alter) @keyword
(keyword_add) @keyword
(keyword_view) @keyword
(keyword_distinct) @keyword
(keyword_union) @keyword
(keyword_case) @keyword
(keyword_when) @keyword
(keyword_then) @keyword
(keyword_else) @keyword
(keyword_end) @keyword
(keyword_with) @keyword
(keyword_in) @keyword
(keyword_is) @keyword
(keyword_like) @keyword
(keyword_between) @keyword
(keyword_inner) @keyword
(keyword_left) @keyword
(keyword_right) @keyword
(keyword_outer) @keyword
(keyword_cross) @keyword
(keyword_full) @keyword
(keyword_having) @keyword
(keyword_limit) @keyword
(keyword_offset) @keyword
(keyword_asc) @keyword
(keyword_desc) @keyword
(keyword_all) @keyword
(keyword_any) @keyword
(keyword_exists) @keyword
(keyword_default) @keyword
(keyword_primary) @keyword
(keyword_foreign) @keyword
(keyword_unique) @keyword
(keyword_check) @keyword
(keyword_constraint) @keyword
(keyword_index) @keyword
(keyword_key) @keyword
(keyword_references) @keyword
(keyword_if) @keyword
(keyword_returning) @keyword
(keyword_cast) @keyword
(keyword_column) @keyword
(keyword_database) @keyword
(keyword_schema) @keyword
(keyword_commit) @keyword
(keyword_rollback) @keyword
(keyword_transaction) @keyword
(keyword_begin) @keyword
(keyword_as) @keyword
(keyword_over) @keyword
(keyword_partition) @keyword
(keyword_window) @keyword

; DDL types
(int) @type
(bigint) @type
(smallint) @type
(tinyint) @type
(decimal) @type
(numeric) @type
(float) @type
(char) @type
(interval) @type

; Identifiers and references
(identifier) @variable
(object_reference) @variable
(column) @variable

; Function calls
(invocation
  unit: (object_reference) @function)