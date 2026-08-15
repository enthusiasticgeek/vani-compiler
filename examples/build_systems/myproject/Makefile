# Mixed C + vāṇī build (tutorials/src/intermediate/09d_build_systems.md's
# "Makefile B" variant): vanic emits C, cc compiles+links against
# c_helper.c. See CMakeLists.txt / meson.build / build.ninja in this
# same directory for the equivalent build with the other three tools --
# all four produce the identical `myproject` binary from the same
# sources.

VANIC ?= vanic
CC    ?= cc
CFLAGS = -O2 -Wall

BUILD    = build-make
VANI_SRCS = $(wildcard src/*.vani)
VANI_C    = $(BUILD)/main_vani.c
VANI_OBJ  = $(BUILD)/main_vani.o
VANI_OBJ_NOMAIN = $(BUILD)/main_vani_nomain.o
C_OBJ     = $(BUILD)/c_helper.o
BIN       = $(BUILD)/myproject

.PHONY: run check build clean

run: build
	./$(BIN)

check:
	$(VANIC) check src/main.vani

build: $(BIN)

# Step 1 -- vāṇी -> C
$(VANI_C): $(VANI_SRCS)
	mkdir -p $(BUILD)
	$(VANIC) emit src/main.vani --backend=c -o $(VANI_C)

# Step 2 -- C -> object
$(VANI_OBJ): $(VANI_C)
	$(CC) $(CFLAGS) -c $(VANI_C) -o $(VANI_OBJ)

# Step 3 -- strip vāṇी's own `main` so it doesn't collide with
# c_helper.c's (vanic emit always lowers `fn main()` to a literal
# `int main(void)`; see src/main.vani's own comment).
$(VANI_OBJ_NOMAIN): $(VANI_OBJ)
	objcopy -N main $(VANI_OBJ) $(VANI_OBJ_NOMAIN)

# Step 4 -- compile the C helper
$(C_OBJ): c_helper.c
	$(CC) $(CFLAGS) -c c_helper.c -o $(C_OBJ)

# Step 5 -- link everything
$(BIN): $(VANI_OBJ_NOMAIN) $(C_OBJ)
	$(CC) $(VANI_OBJ_NOMAIN) $(C_OBJ) -o $(BIN)

clean:
	rm -rf $(BUILD)
