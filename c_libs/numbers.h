#include <float.h>
#include <math.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

//! The Integer class is a 64 bit integer
typedef struct {
    int64_t value;
} Integer;

//! The Float class is a 64 bit float
typedef struct {
    double value;
} Float;

//! Integer value from C integer
Integer integer_from(int64_t value) {
    Integer i = {
        .value = value
    };
    return i;
}

//! Float value from C double
Float float_from(double value) {
    Float f = {
        .value = value
    };
    return f;
}

// -------------------- Traits --------------------

const size_t INTEGER_BUFFER_SIZE = 21;
const size_t FLOAT_BUFFER_SIZE = 64;

//! Convert Integer to a C string
char* integer_show(Integer num) {
    char* str = (char*)malloc(INTEGER_BUFFER_SIZE);
    if (str == NULL) {
        perror("Fatal runtime error\nFailed to allocate memory\nTried to represent an integer with a string\nCalling code: `integer_show`");
        exit(EXIT_FAILURE);
    }
    snprintf(str, INTEGER_BUFFER_SIZE, "%" PRId64, num.value);
    return str;
}

//! Convert Float to a C string
char* float_show(Float num) {
    char* str = (char*)malloc(FLOAT_BUFFER_SIZE);
    if (str == NULL) {
        perror("Fatal runtime error\nFailed to allocate memory\nTried to represent an integer with a string\nCalling code: `integer_show`");
        exit(EXIT_FAILURE);
    }
    snprintf(str, FLOAT_BUFFER_SIZE, "%.17g", num.value); // %.17g for compact representation of doubles
    return str;
}

//! Checks if two integers are equal
bool integer_equals(Integer a, Integer b) {
    return a.value == b.value;
}

//! Checks if two floats are equal
//! 
//! TODO: improve
bool float_equals(Float a, Float b) {
    if (a.value == b.value) {
        return true;
    } else {
        return (fabs(a.value) - fabs(b.value)) < DBL_EPSILON;
    }
}

// -------------------- Basic Arithmetic --------------------

//! Saturating addition for Integer
Integer saturating_add(Integer a, Integer b) {
    if (a.value > 0) {
        if (b.value > INT64_MAX - a.value) {
            return integer_from(INT64_MAX);
        }
    } else if (b.value < INT64_MIN - a.value) {
        return integer_from(INT64_MIN);
    }
    return integer_from(a.value + b.value);
}

//! Saturating subtraction for Integer
Integer saturating_sub(Integer a, Integer b) {
    if (b.value < 0) {
        if (a.value > INT64_MAX + b.value) {
            return integer_from(INT64_MAX);
        }
    } else if (a.value < INT64_MIN + b.value) {
        return integer_from(INT64_MIN);
    }
    return integer_from(a.value - b.value);
}

//! Saturating multiplication for Integer
Integer saturating_mul(Integer a, Integer b) {
    if (a.value > 0) {
        if (b.value > 0 && a.value > INT64_MAX / b.value) {
            return integer_from(INT64_MAX);
        } else if (b.value < 0 && b.value < INT64_MIN / a.value) {
            return integer_from(INT64_MIN);
        }
    } else if (a.value < 0) {
        if (b.value > 0 && a.value < INT64_MIN / b.value) {
            return integer_from(INT64_MIN);
        } else if (b.value < 0 && a.value < INT64_MAX / b.value) {
            return integer_from(INT64_MAX);
        }
    }
    return integer_from(a.value * b.value);
}

//! Saturating division for Integer
Integer saturating_div(Integer a, Integer b) {
    if (b.value == 0) {
        // Division by zero is undefined; return the maximum value as a fallback.
        return integer_from(a.value > 0 ? INT64_MAX : INT64_MIN);
    }
    if (a.value == INT64_MIN && b.value == -1) {
        // Special case: overflow when dividing INT64_MIN by -1
        return integer_from(INT64_MAX);
    }
    return integer_from(a.value / b.value);
}

//! Clamp helper function for Float
static double clamp(double value, double min, double max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

//! Saturating addition for Float
Float saturating_add_float(Float a, Float b) {
    double result = a.value + b.value;
    return float_from(clamp(result, -DBL_MAX, DBL_MAX));
}

//! Saturating subtraction for Float
Float saturating_sub_float(Float a, Float b) {
    double result = a.value - b.value;
    return float_from(clamp(result, -DBL_MAX, DBL_MAX));
}

//! Saturating multiplication for Float
Float saturating_mul_float(Float a, Float b) {
    double result = a.value * b.value;
    return float_from(clamp(result, -DBL_MAX, DBL_MAX));
}

//! Saturating division for Float
Float saturating_div_float(Float a, Float b) {
    if (b.value == 0.0) {
        // Division by zero; saturate to maximum value
        return float_from(a.value > 0 ? DBL_MAX : -DBL_MAX);
    }
    double result = a.value / b.value;
    return float_from(clamp(result, -DBL_MAX, DBL_MAX));
}