// source: ./stdlib/core.iona

#include <stdbool.h>
#include <stdint.h>

typedef enum {
	SOME,
	NONE,
} MaybeStates;

typedef union {
	void* Some;
} MaybeValues;

struct Maybe {
	MaybeStates tag;
	MaybeValues data;
};

typedef enum {
	OKAY,
	ERROR,
} ResultStates;

typedef union {
	void* Okay;
	void* Error;
} ResultValues;

struct Result {
	ResultStates tag;
	ResultValues data;
};

