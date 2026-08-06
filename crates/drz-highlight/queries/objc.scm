; Objective-C syntax highlighting using tree-sitter-objc 3.0.2 grammar node names

; Comments and preprocessor
(comment) @comment
(preproc_call) @keyword
(preproc_directive) @keyword
(preproc_arg) @string
(preproc_include) @keyword
(preproc_def) @keyword
(preproc_function_def) @keyword

; String and number literals
(string_literal) @string
(concatenated_string) @string
(number_literal) @number
(char_literal) @string
(null) @constant
(true) @constant
(false) @constant

; Objective-C @ directives and C keywords
"@interface" @keyword
"@implementation" @keyword
"@protocol" @keyword
"@end" @keyword
"@property" @keyword
"@synthesize" @keyword
"@dynamic" @keyword
"@selector" @keyword
"@autoreleasepool" @keyword
"@try" @keyword
"@catch" @keyword
"@throw" @keyword
"@finally" @keyword
"@synchronized" @keyword
"@public" @keyword
"@private" @keyword
"@protected" @keyword
"@package" @keyword
"@optional" @keyword
"@required" @keyword
"@encode" @keyword
"@defs" @keyword
"@compatibility_alias" @keyword

; C keywords (anonymous tokens)
"if" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"do" @keyword
"switch" @keyword
"case" @keyword
"default" @keyword
"return" @keyword
"break" @keyword
"continue" @keyword
"goto" @keyword
"sizeof" @keyword
"typedef" @keyword
"struct" @keyword
"union" @keyword
"enum" @keyword
"static" @keyword
"const" @keyword
"extern" @keyword
"inline" @keyword
"short" @keyword
"long" @keyword
"signed" @keyword
"unsigned" @keyword
"register" @keyword
"volatile" @keyword
"auto" @keyword

; Types and identifiers
(primitive_type) @type
(type_identifier) @type
(identifier) @variable

; Function declarations and calls
(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression) @function)

; Field access
(field_expression
  field: (_) @variable)