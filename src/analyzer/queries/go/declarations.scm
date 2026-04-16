(function_declaration
  name: (identifier) @fn_name) @fn

(method_declaration
  receiver: (parameter_list
    (parameter_declaration
      type: [
        (type_identifier) @recv_type
        (pointer_type (type_identifier) @recv_type)
      ]))
  name: (field_identifier) @method_name) @method

(type_spec
  name: (type_identifier) @struct_name
  type: (struct_type)) @struct_decl

(type_spec
  name: (type_identifier) @iface_name
  type: (interface_type)) @iface_decl

(type_spec
  name: (type_identifier) @type_name) @type_decl

(type_alias
  name: (type_identifier) @alias_name) @alias_decl