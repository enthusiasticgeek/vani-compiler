# Mixed C + vāṇी build (tutorials/src/intermediate/09d_build_systems.md).
# See Makefile / CMakeLists.txt / build.ninja in this same directory
# for the equivalent build with the other three tools -- all four
# produce the identical `myproject` binary from the same sources.
#
#   meson setup build-meson && ninja -C build-meson && ./build-meson/myproject

project('myproject', 'c',
  default_options: ['c_std=c11', 'optimization=2'])

vanic = find_program('vanic', required: true)
objcopy = find_program('objcopy', required: true)

# Step 1 -- vāṇी -> C
vani_c = custom_target('vani_emit',
  input:   'src/main.vani',
  output:  'main_vani.c',
  command: [vanic, 'emit', '@INPUT@', '--backend=c', '-o', '@OUTPUT@'],
  depend_files: files('src/math.vani'),   # re-emit when modules change
)

# Step 2 -- strip vāṇी's own `main` so it doesn't collide with
# c_helper.c's (vanic emit always lowers `fn main()` to a literal
# `int main(void)`; see src/main.vani's own comment). Meson's
# static_library() doesn't expose its compiled .o path for a
# post-processing step like objcopy, so compile the vāṇी C output
# to a plain object file directly via custom_target instead of
# static_library() + extraction.
vani_obj = custom_target('vani_compile_obj',
  input:   vani_c,
  output:  'main_vani.o',
  command: [meson.get_compiler('c').cmd_array(), '-O2', '-c', '@INPUT@', '-o', '@OUTPUT@'],
)
vani_obj_nomain = custom_target('vani_strip_main',
  input:   vani_obj,
  output:  'main_vani_nomain.o',
  command: [objcopy, '-N', 'main', '@INPUT@', '@OUTPUT@'],
)

# Step 3 -- link the C side against the stripped object.
executable('myproject',
  sources: ['c_helper.c', vani_obj_nomain],
)
