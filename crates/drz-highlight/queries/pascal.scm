; Pascal / Delphi — adapted from tree-sitter-pascal upstream highlights.scm,
; mapped to drz-highlight's plain capture set (keyword, string, comment,
; function, type, number, constant).

(comment) @comment
(pp) @keyword

(literalString) @string
(literalNumber) @number
(literalChar) @string

[
  (kProgram) (kLibrary) (kUnit) (kUses)
  (kBegin) (kEnd) (kAsm)
  (kVar) (kThreadvar) (kConst) (kResourcestring) (kConstref) (kOut)
  (kType) (kLabel) (kExports)
  (kAbsolute)
  (kProperty) (kRead) (kWrite) (kImplements) (kDefault) (kNodefault)
  (kStored) (kIndex) (kDispId)
  (kClass) (kInterface) (kDispInterface) (kObject) (kRecord) (kArray)
  (kFile) (kString) (kSet) (kOf) (kHelper) (kPacked)
  (kGeneric) (kSpecialize)
  (kFunction) (kProcedure) (kConstructor) (kDestructor) (kOperator) (kReference)
  (kImplementation) (kInitialization) (kFinalization)
  (kPublished) (kPublic) (kProtected) (kPrivate) (kStrict)
  (kRequired) (kOptional)
  (kForward)
  (kStatic) (kVirtual) (kAbstract) (kSealed) (kDynamic) (kOverride)
  (kOverload) (kReintroduce) (kInherited) (kInline)
  (kStdcall) (kCdecl) (kPascal) (kRegister) (kExternal) (kName)
  (kMessage) (kDeprecated) (kExperimental) (kPlatform) (kUnimplemented)
  (kFar) (kNear) (kSafecall) (kAssembler) (kInterrupt) (kNoreturn)
  (kVarargs) (kWinapi) (kAlias) (kDelayed)
  (kFor) (kTo) (kDownto) (kIf) (kThen) (kElse) (kDo) (kWhile)
  (kRepeat) (kUntil) (kTry) (kExcept) (kFinally) (kRaise) (kOn)
  (kCase) (kWith) (kGoto)
] @keyword

[
  (kOr) (kXor) (kDiv) (kMod) (kAnd) (kShl) (kShr) (kNot) (kIs) (kAs) (kIn)
] @keyword

[
  (kTrue) (kFalse)
] @constant

(declType name: (identifier) @type)
(declType name: (genericTpl entity: (identifier) @type))

(declProc name: (identifier) @function)
(declProc name: (genericTpl entity: (identifier) @function))
(declProc name: (genericDot rhs: (identifier) @function))
(declProc name: (genericDot rhs: (genericTpl entity: (identifier) @function)))

(declProp name: (identifier) @function)

(exprCall entity: (identifier) @function)
(exprCall entity: (exprTpl entity: (identifier) @function))
(exprCall entity: (exprDot rhs: (identifier) @function))
(exprCall entity: (exprDot rhs: (exprTpl entity: (identifier) @function)))

(typeref) @type