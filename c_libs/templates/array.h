// Template parameters to be replaced by compiler:
// ARRAY_NAME -> concrete type name (e.g., StringArray, IntArray)
// ELEM_TYPE -> concrete element type (e.g., char, int)
// PREFIX -> function prefix (e.g., string_array, int_array)
// OTHER_IMPORTS -> what other packages do we need?

#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
<OTHER_IMPORTS>
typedef struct {
    ELEM_TYPE* data;
    size_t len;
    size_t capacity;
} ARRAY_NAME;

// Create a new empty array with default capacity
ARRAY_NAME PREFIX_new(void) {
    const size_t initial_capacity = 8;
    ARRAY_NAME arr = {
        .data = malloc(sizeof(ELEM_TYPE) * initial_capacity),
        .len = 0,
        .capacity = initial_capacity
    };
    return arr;
}

// Create array with specific capacity
ARRAY_NAME PREFIX_with_capacity(size_t capacity) {
    ARRAY_NAME arr = {
        .data = malloc(sizeof(ELEM_TYPE) * capacity),
        .len = 0,
        .capacity = capacity
    };
    return arr;
}

// Free the array's memory
void PREFIX_free(ARRAY_NAME* arr) {
    free(arr->data);
    arr->data = NULL;
    arr->len = 0;
    arr->capacity = 0;
}

// Ensure the array has enough capacity for additional elements
void PREFIX_reserve(ARRAY_NAME* arr, size_t additional) {
    size_t required = arr->len + additional;
    if (required <= arr->capacity) return;
    
    // Grow by doubling or required amount, whichever is larger
    size_t new_capacity = arr->capacity * 2;
    if (new_capacity < required) new_capacity = required;
    
    ELEM_TYPE* new_buf = realloc(arr->data, sizeof(ELEM_TYPE) * new_capacity);
    arr->data = new_buf;
    arr->capacity = new_capacity;
}

// Push an element to the end
void PREFIX_push(ARRAY_NAME* arr, ELEM_TYPE elem) {
    PREFIX_reserve(arr, 1);
    arr->data[arr->len++] = elem;
}

// Pop an element from the end
ELEM_TYPE PREFIX_pop(ARRAY_NAME* arr) {
    if (arr->len == 0) {
        // TODO: use Result type
        ELEM_TYPE zero = {0};  // Zero initialization works for most types
        return zero;
    }
    return arr->data[--arr->len];
}

// Get a slice of the array (returns new array)
ARRAY_NAME PREFIX_slice(const ARRAY_NAME* arr, size_t start, size_t end) {
    if (end > arr->len) end = arr->len;
    if (start > end) start = end;
    
    size_t slice_len = end - start;
    ARRAY_NAME result = PREFIX_with_capacity(slice_len);
    
    memcpy(result.data, arr->data + start, slice_len * sizeof(ELEM_TYPE));
    result.len = slice_len;
    
    return result;
}

// Get element at index (bounds checking optional based on your language's semantics)
ELEM_TYPE PREFIX_get(const ARRAY_NAME* arr, size_t index) {
    if (index >= arr->len) {
        // Handle out of bounds - TODO: use Result type
        ELEM_TYPE zero = {0};
        return zero;
    }
    return arr->data[index];
}

// Set element at index (bounds checking optional based on your language's semantics)
bool PREFIX_set(ARRAY_NAME* arr, size_t index, ELEM_TYPE elem) {
    if (index >= arr->len) {
        return false;  // Or handle error based on your language's semantics
    }
    arr->data[index] = elem;
    return true;
}