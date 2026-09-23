#ifndef DIFFLOOM_H
#define DIFFLOOM_H

#ifdef __cplusplus
extern "C" {
#endif

int diffloom_abi_version(void);

char *diffloom_diff(const char *old_text, const char *new_text);
char *diffloom_patch_make(const char *old_text, const char *new_text);
char *diffloom_patch_apply(const char *patch_text, const char *input_text);

void diffloom_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif
