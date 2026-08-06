; Julia — adapted from tree-sitter-julia upstream highlights.scm,
; mapped to drz-highlight's plain capture set (keyword, string, comment,
; function, type, number, constant).

[
  (line_comment)
  (block_comment)
] @comment

(string_literal) @string
(character_literal) @string
(command_literal) @string

[
  (integer_literal)
  (float_literal)
] @number

(boolean_literal) @constant

(string_literal) @string

; Function calls + broadcasts
(call_expression (identifier) @function)
(call_expression (field_expression (identifier) @function .))
(broadcast_call_expression (identifier) @function)

; Macro invocations
(macro_identifier) @function

; Type annotations
(typed_expression (identifier) @type .)
(unary_typed_expression (identifier) @type .)
(parametrized_type_expression (identifier) @type)
(parametrized_type_expression (field_expression (identifier) @type .))

; Type definitions
(type_head (_) @type)

; Keyword operators
((operator) @keyword
  (#any-of? @keyword "in" "isa"))

(where_expression "where" @keyword)

; Statement-level keywords via parent context
(if_statement
  [
    "if"
    "end"
  ] @keyword)

(elseif_clause "elseif" @keyword)
(else_clause "else" @keyword)

(try_statement
  [
    "try"
    "end"
  ] @keyword)
(catch_clause "catch" @keyword)
(finally_clause "finally" @keyword)

(for_statement
  [
    "for"
    "end"
  ] @keyword)
(while_statement
  [
    "while"
    "end"
  ] @keyword)
(do_clause
  [
    "do"
    "end"
  ] @keyword)

(function_definition
  [
    "function"
    "end"
  ] @keyword)
(return_statement "return" @keyword)

(import_statement "import" @keyword)
(using_statement "using" @keyword)
(export_statement "export" @keyword)
(import_alias "as" @keyword)

(struct_definition
  [
    "mutable"
    "struct"
    "end"
  ] @keyword)
(abstract_definition
  [
    "abstract"
    "type"
    "end"
  ] @keyword)

(compound_statement
  [
    "begin"
    "end"
  ] @keyword)
(let_statement
  [
    "let"
    "end"
  ] @keyword)
(module_definition
  [
    "module"
    "baremodule"
    "end"
  ] @keyword)

[
  "const"
  "global"
  "local"
  "macro"
  "primitive"
  "quote"
] @keyword