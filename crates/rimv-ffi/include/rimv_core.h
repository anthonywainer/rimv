#ifndef RIMV_CORE_H
#define RIMV_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RimvEngine RimvEngine;

/* UTF-8 JSON string; caller frees all returned strings with rimv_string_free. */
uint32_t rimv_api_version(void);
char *rimv_engine_create(const char *config_json, RimvEngine **out_engine);
char *rimv_engine_request(RimvEngine *engine, const char *request_json);
void rimv_string_free(char *value);
void rimv_engine_destroy(RimvEngine *engine);

#ifdef __cplusplus
}
#endif

#endif
