#pragma once
#include <cstddef>
#include <cstdint>
#include <stdbool.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct EngineHandle EngineHandle;
EngineHandle *omarchy_ime_engine_new(void);
void omarchy_ime_engine_free(EngineHandle *);
int omarchy_ime_engine_key(EngineHandle *,uint32_t,uint32_t,bool);
int omarchy_ime_engine_select(EngineHandle *,size_t);
size_t omarchy_ime_engine_candidate_count(const EngineHandle *);
const char *omarchy_ime_engine_candidate(const EngineHandle *,size_t);
const char *omarchy_ime_engine_preedit(const EngineHandle *);
const char *omarchy_ime_engine_commit(const EngineHandle *);
#ifdef __cplusplus
}
#endif
