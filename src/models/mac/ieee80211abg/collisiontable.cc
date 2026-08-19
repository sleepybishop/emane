
#include "collisiontable.h"

extern "C" {
    void* emane_ieee80211abg_collisiontable_new();
    void emane_ieee80211abg_collisiontable_free(void* ptr);
    float emane_ieee80211abg_collisiontable_getCollisionFactor(void* ptr, int num, int cw);
}

EMANE::Models::IEEE80211ABG::CollisionTable::CollisionTable() {}

float EMANE::Models::IEEE80211ABG::CollisionTable::getCollisionFactor(int num, int cw)
{
    return emane_ieee80211abg_collisiontable_getCollisionFactor(nullptr, num, cw);
}

float EMANE::Models::IEEE80211ABG::CollisionTable::interpolate(float x0, float x1, float y0, float y1, float x)
{
    return 0.0f;
}
