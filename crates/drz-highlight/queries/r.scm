; R syntax highlighting using tree-sitter-r 1.3.0 grammar node names

; Comments and strings
(comment) @comment
(string) @string

; Numbers and constants
(integer) @number
(float) @number
(complex) @number
(true) @constant
(false) @constant
(null) @constant
(na) @constant
(nan) @constant
(inf) @constant

; R keywords (anonymous tokens)
"if" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"function" @keyword
"repeat" @keyword
"in" @keyword
"NA" @constant

; Control flow statements (named nodes)
(break) @keyword
(next) @keyword
(if_statement) @keyword
(for_statement) @keyword
(while_statement) @keyword
(repeat_statement) @keyword

; Function calls - name is identifier or other expression
(call
  function: (identifier) @function)

; Identifiers
(identifier) @variable

; Parameters
(parameters
  (parameter
    name: (identifier) @variable))

; Operators
(binary_operator) @operator
(unary_operator) @operator
(extract_operator) @operator
(subset) @operator
(subset2) @operator
(namespace_operator) @operator