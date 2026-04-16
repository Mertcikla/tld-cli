(function_definition
  declarator: _ @method_declarator) @method_def

(field_declaration
  declarator: (function_declarator
    declarator: _ @decl_member))

(declaration
  declarator: (function_declarator
    declarator: _ @ctor_decl_member))