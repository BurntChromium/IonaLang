CHECK_FLAGS = -Wall -Werror -Wextra -std=c99 -pedantic -fsyntax-only

# Check files
check-builtin:
	gcc bytes.h $(CHECK_FLAGS)
	gcc console_io.h $(CHECK_FLAGS)
	gcc numbers.h $(CHECK_FLAGS)

# Check files emitted by the Iona compiler
check-generated:
	$(foreach file, $(wildcard gen_*.h), gcc $(file) $(CHECK_FLAGS);)
