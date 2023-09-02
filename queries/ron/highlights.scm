
(enum_variant) @constant
(struct_entry (identifier) @variable.other.member)
(struct_entry (enum_variant (identifier) @constant))
(struct_name (identifier)) @type

(unit_struct) @type.builtin

(escape_sequence) @constant.character.escape
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(char) @constant.character
[
  (string)
  (raw_string)
] @string
[
  (line_comment)
  (block_comment)
] @comment

; ---
; Extraneous
; ---

(enum_variant (identifier) @type.enum.variant)

; ---
; Punctuation
; ---

["{" "}"] @punctuation.bracket

["(" ")"] @punctuation.bracket

["[" "]"] @punctuation.bracket

[
  ","
  ":"
] @punctuation.delimiter

(ERROR) @error
