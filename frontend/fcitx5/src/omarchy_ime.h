#pragma once
#include <cstddef>
#include <cstdint>
#include <stdbool.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct EngineHandle EngineHandle;
enum { OMARCHY_IME_KEY_BACKSPACE=0x110000, OMARCHY_IME_KEY_ENTER=0x110001, OMARCHY_IME_KEY_ESCAPE=0x110002, OMARCHY_IME_KEY_SPACE=0x110003 };
EngineHandle *omarchy_ime_engine_new(void);
int omarchy_ime_engine_set_mode(EngineHandle *,int);
int omarchy_ime_engine_mode(const EngineHandle *);
bool omarchy_ime_engine_wants_key(const EngineHandle *,uint32_t);
void omarchy_ime_engine_free(EngineHandle *);
int omarchy_ime_engine_key(EngineHandle *,uint32_t,uint32_t,bool);
int omarchy_ime_engine_set_surrounding(EngineHandle *,const uint8_t *,size_t,size_t);
int omarchy_ime_engine_flush(EngineHandle *);
int omarchy_ime_engine_select(EngineHandle *,size_t);
size_t omarchy_ime_engine_candidate_count(const EngineHandle *);
const char *omarchy_ime_engine_candidate(const EngineHandle *,size_t);
const char *omarchy_ime_engine_preedit(const EngineHandle *);
const char *omarchy_ime_engine_commit(const EngineHandle *);
#ifdef __cplusplus
}
#endif
