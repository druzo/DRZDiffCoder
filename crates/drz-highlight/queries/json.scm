; JSON — adapted from tree-sitter-json upstream highlights.scm,
; mapped to drz-highlight's plain capture set (keyword, string, comment,
; function, type, number, constant).

(string) @string

(number) @number

[
  (null)
  (true)
  (false)
] @constant

(escape_sequence) @string

(pair
  key: (string) @keyword)

(comment) @comment