// source: ./test_programs/comprehensive.iona

#include <stdbool.h>
#include <stdint.h>

struct Animal {
	int_fast64_t legs;
	bool hair;
	bool feathers;
};
typedef struct Animal Animal;

typedef enum {
	DOG,
	FISH,
	BIRD,
	CAT,
} PetsStates;

typedef union {
	int_fast64_t Cat;
} PetsValues;

struct Pets {
	PetsStates tag;
	PetsValues data;
};
typedef struct Pets Pets;

void print_pet(Pets pet);