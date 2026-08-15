# Mixed C + vāṇी build (tutorials/src/intermediate/09d_build_systems.md).
# See Makefile / meson.build / build.ninja in this same directory for
# the equivalent build with the other three tools -- all four produce
# the identical `myproject` binary from the same sources.
#
#   cmake -B build-cmake && cmake --build build-cmake && ./build-cmake/myproject
#   cmake -B build-cmake -G Ninja && cmake --build build-cmake   # Ninja generator, same CMakeLists.txt

cmake_minimum_required(VERSION 3.20)
project(myproject LANGUAGES C)

find_program(VANIC vanic REQUIRED
  DOC "vāṇī compiler CLI (install via cargo install vanic, or point -DVANIC=/path/to/vanic)")
find_program(OBJCOPY objcopy REQUIRED)

set(VANI_ENTRY ${CMAKE_SOURCE_DIR}/src/main.vani)
set(VANI_C_OUT ${CMAKE_BINARY_DIR}/main_vani.c)

file(GLOB_RECURSE VANI_SRCS ${CMAKE_SOURCE_DIR}/src/*.vani)

# Step 1 -- vāṇी -> C
add_custom_command(
  OUTPUT  ${VANI_C_OUT}
  COMMAND ${VANIC} emit ${VANI_ENTRY} --backend=c -o ${VANI_C_OUT}
  DEPENDS ${VANI_SRCS}
  COMMENT "vani-emit ${VANI_ENTRY}"
  VERBATIM
)

# Step 2 -- compile the generated C as a static library
add_library(myproject_vani STATIC ${VANI_C_OUT})
target_compile_options(myproject_vani PRIVATE -O2)

# Step 3 -- strip vāṇी's own `main` so it doesn't collide with
# c_helper.c's (vanic emit always lowers `fn main()` to a literal
# `int main(void)`; see src/main.vani's own comment).
set(VANI_OBJ_NOMAIN ${CMAKE_BINARY_DIR}/main_vani_nomain.o)
add_custom_command(
  OUTPUT  ${VANI_OBJ_NOMAIN}
  COMMAND ${OBJCOPY} -N main $<TARGET_OBJECTS:myproject_vani> ${VANI_OBJ_NOMAIN}
  DEPENDS myproject_vani
  COMMENT "strip vani's main so it doesn't collide with c_helper.c's"
)
add_custom_target(myproject_vani_nomain DEPENDS ${VANI_OBJ_NOMAIN})

# Step 4 -- your C executable links against the stripped object directly
# (not against myproject_vani, which still has the colliding main).
add_executable(myproject c_helper.c ${VANI_OBJ_NOMAIN})
add_dependencies(myproject myproject_vani_nomain)
