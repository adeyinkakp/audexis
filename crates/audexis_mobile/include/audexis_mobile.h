#ifndef AUDEXIS_MOBILE_H
#define AUDEXIS_MOBILE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MobileCore MobileCore;

MobileCore *audexis_mobile_create(void);

char *audexis_mobile_call(MobileCore *core, const char *request);
void audexis_mobile_
(char *value);
void audexis_mobile_destroy(MobileCore *core);

#ifdef __cplusplus
}
#endif
#endif
