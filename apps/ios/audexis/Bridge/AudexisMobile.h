#ifndef AUDEXIS_MOBILE_H
#define AUDEXIS_MOBILE_H

typedef struct MobileCore MobileCore;
MobileCore *audexis_mobile_create(void);
char *audexis_mobile_call(MobileCore *core, const char *request);
void audexis_mobile_destroy(MobileCore *core);
void audexis_mobile_string_free(char *value);

#endif
