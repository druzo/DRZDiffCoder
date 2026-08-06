; Scala syntax highlighting using tree-sitter-scala 0.26.0 grammar node names

; Comments and strings
(comment) @comment
(block_comment) @comment
(string) @string

; Numbers and constants
(boolean_literal) @constant
(integer_literal) @number
(floating_point_literal) @number

; Scala keywords (anonymous tokens)
"abstract" @keyword
"case" @keyword
"catch" @keyword
"class" @keyword
"def" @keyword
"do" @keyword
"else" @keyword
"extends" @keyword
"finally" @keyword
"for" @keyword
"if" @keyword
"implicit" @keyword
"import" @keyword
"lazy" @keyword
"match" @keyword
"new" @keyword
"object" @keyword
"override" @keyword
"package" @keyword
"private" @keyword
"protected" @keyword
"return" @keyword
"sealed" @keyword
"throw" @keyword
"trait" @keyword
"try" @keyword
"type" @keyword
"val" @keyword
"var" @keyword
"while" @keyword
"with" @keyword
"yield" @keyword
"this" @keyword

; Type and identifier references
(type_identifier) @type
(identifier) @variable

; Function definitions and calls
(call_expression
  function: (_) @function)
(class_definition
  name: (_) @type)
(trait_definition
  name: (_) @type)
(object_definition
  name: (_) @type)