(class_specifier
  name: (type_identifier) @class_name
  body: (field_declaration_list) @class_body) @class_decl

(struct_specifier
  name: (type_identifier) @struct_name
  body: (field_declaration_list) @struct_body) @struct_decl

(enum_specifier
  name: (type_identifier) @enum_name) @enum_decl

(function_definition
  declarator: _ @fn_declarator) @fn_def