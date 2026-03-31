(module
  ;; Extism host imports (must come first)
  (import "extism:host/env" "input_length" (func $input_length (result i64)))
  (import "extism:host/env" "input_load_u8" (func $input_load_u8 (param i64) (result i32)))
  (import "extism:host/env" "output_set" (func $output_set (param i64 i64)))
  (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
  (import "extism:host/env" "store_u8" (func $store_u8 (param i64 i32)))

  ;; Memory for Extism
  (memory (export "memory") 2)

  ;; Data section: "Hello, " string at offset 1024
  (data (i32.const 1024) "Hello, ")

  ;; greet function: takes name input, returns "Hello, {name}!"
  (func (export "greet") (result i32)
    (local $in_len i64)
    (local $result_off i64)
    (local $total i64)
    (local $i i64)

    ;; Read input length
    (local.set $in_len (call $input_length))

    ;; Total length: "Hello, " (7) + input + "!" (1)
    (local.set $total (i64.add (i64.add (i64.const 7) (local.get $in_len)) (i64.const 1)))

    ;; Allocate result buffer
    (local.set $result_off (call $alloc (local.get $total)))

    ;; Copy "Hello, " (7 bytes) to result
    (local.set $i (i64.const 0))
    (block $b1 (loop $l1
      (br_if $b1 (i64.ge_u (local.get $i) (i64.const 7)))
      (call $store_u8
        (i64.add (local.get $result_off) (local.get $i))
        (i32.load8_u (i32.add (i32.const 1024) (i32.wrap_i64 (local.get $i))))
      )
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $l1)
    ))

    ;; Copy input name to result (after "Hello, ")
    (local.set $i (i64.const 0))
    (block $b2 (loop $l2
      (br_if $b2 (i64.ge_u (local.get $i) (local.get $in_len)))
      (call $store_u8
        (i64.add (i64.add (local.get $result_off) (i64.const 7)) (local.get $i))
        (call $input_load_u8 (local.get $i))
      )
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $l2)
    ))

    ;; Append "!" (1 byte)
    (call $store_u8
      (i64.add (local.get $result_off) (i64.sub (local.get $total) (i64.const 1)))
      (i32.const 33)  ;; '!'
    )

    ;; Set output
    (call $output_set (local.get $result_off) (local.get $total))

    (i32.const 0)
  )

  ;; Simple echo function for basic testing
  (func (export "echo") (result i32)
    (local $len i64)
    (local $off i64)
    (local $i i64)

    (local.set $len (call $input_length))
    (local.set $off (call $alloc (local.get $len)))

    ;; Copy input to output
    (local.set $i (i64.const 0))
    (block $break (loop $loop
      (br_if $break (i64.ge_u (local.get $i) (local.get $len)))
      (call $store_u8
        (i64.add (local.get $off) (local.get $i))
        (call $input_load_u8 (local.get $i))
      )
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $loop)
    ))

    (call $output_set (local.get $off) (local.get $len))
    (i32.const 0)
  )
)
