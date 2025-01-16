// source: test_programs/comprehensive.iona

#include <stdbool.h>
#include "../c_libs/numbers.h"

struct Animal {
	Integer legs;
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
	Integer Cat;
} PetsValues;

struct Pets {
	PetsStates tag;
	PetsValues data;
};
typedef struct Pets Pets;

void print_pet(Pets pet);