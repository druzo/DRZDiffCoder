; PHP syntax highlighting using tree-sitter-php 0.24.2 grammar node names

; Comments and strings
(comment) @comment
(string) @string
(encapsed_string) @string
(heredoc) @string
(nowdoc) @string
(shell_command_expression) @string

; Numbers and constants
(integer) @number
(float) @number
(boolean) @constant
(null) @constant

; PHP keywords (anonymous tokens)
"if" @keyword
"else" @keyword
"elseif" @keyword
"for" @keyword
"foreach" @keyword
"while" @keyword
"do" @keyword
"switch" @keyword
"case" @keyword
"default" @keyword
"break" @keyword
"continue" @keyword
"return" @keyword
"function" @keyword
"class" @keyword
"interface" @keyword
"trait" @keyword
"extends" @keyword
"implements" @keyword
"abstract" @keyword
"final" @keyword
"public" @keyword
"private" @keyword
"protected" @keyword
"static" @keyword
"const" @keyword
"namespace" @keyword
"use" @keyword
"new" @keyword
"try" @keyword
"catch" @keyword
"finally" @keyword
"throw" @keyword
"echo" @keyword
"print" @keyword
"exit" @keyword
"list" @keyword
"array" @keyword
"as" @keyword
"global" @keyword
"instanceof" @keyword
"insteadof" @keyword
"goto" @keyword
"declare" @keyword
"enddeclare" @keyword
"endforeach" @keyword
"endfor" @keyword
"endif" @keyword
"endswitch" @keyword
"endwhile" @keyword
"match" @keyword
"fn" @keyword
"readonly" @keyword
"enum" @keyword

; Types
(primitive_type) @type
(name) @variable

; Function and class declarations
(method_declaration
  name: (_) @function)
(class_declaration
  name: (_) @type)
(function_definition
  name: (_) @function)
(interface_declaration
  name: (_) @type)
(trait_declaration
  name: (_) @type)
(enum_declaration
  name: (_) @type)

; Function calls
(function_call_expression
  function: (_) @function)
(member_call_expression
  name: (_) @function)

; Variables
(variable_name) @variable