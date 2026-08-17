#include <cstdint>
#include "spectralmaskmanager.h"
#include "frequencyoverlapratio.h"
#include "emane/spectralmaskexception.h"

extern "C" {
    struct FfiSpectralSegment {
        double overlap_ratio;
        double modifier_mw;
        uint64_t lower_hz;
        uint64_t upper_hz;
    };

    struct FfiSpectralOverlap {
        FfiSpectralSegment* segments;
        size_t segments_len;
        uint64_t lower_hz;
        uint64_t upper_hz;
    };

    struct FfiMaskOverlap {
        FfiSpectralOverlap* overlaps;
        size_t overlaps_len;
        uint64_t lower_hz;
        uint64_t upper_hz;
        uint64_t total;
    };

    void emane_rs_spectral_mask_load(const char* uri, char* error_buf, size_t error_buf_len);
    uint64_t emane_rs_spectral_mask_get_primary_bandwidth(uint16_t mask_id);
    bool emane_rs_spectral_mask_get_overlap(uint64_t tx_freq, uint64_t rx_freq, uint64_t rx_bw, uint64_t tx_bw, uint16_t mask_id, FfiMaskOverlap* out);
    void emane_rs_spectral_mask_free_overlap(FfiMaskOverlap* overlap);
}

void EMANE::SpectralMaskManager::load(const std::string & sSpectralMaskManifestURI)
{
    char error_buf[1024];
    error_buf[0] = 0;
    emane_rs_spectral_mask_load(sSpectralMaskManifestURI.c_str(), error_buf, sizeof(error_buf));
    if (error_buf[0] != 0) {
        throw SpectralMaskException(error_buf);
    }
}

EMANE::SpectralMaskManager::MaskOverlap
EMANE::SpectralMaskManager::getSpectralOverlap(std::uint64_t u64TxFrequencyHz,
                                               std::uint64_t u64RxFrequencyHz,
                                               std::uint64_t u64RxBandwidthHz,
                                               std::uint64_t u64TxBandwidthHz,
                                               std::uint16_t u16SpectalMaskId) const
{
    FfiMaskOverlap out;
    if (emane_rs_spectral_mask_get_overlap(u64TxFrequencyHz, u64RxFrequencyHz, u64RxBandwidthHz, u64TxBandwidthHz, u16SpectalMaskId, &out)) {
        SpectralOverlaps spectralOverlaps;
        
        for (size_t i = 0; i < out.overlaps_len; ++i) {
            FfiSpectralOverlap* ov = &out.overlaps[i];
            SpectralSegments spectralSegments;
            
            for (size_t j = 0; j < ov->segments_len; ++j) {
                FfiSpectralSegment* seg = &ov->segments[j];
                spectralSegments.emplace_back(std::make_tuple(
                    seg->overlap_ratio,
                    seg->modifier_mw,
                    seg->lower_hz,
                    seg->upper_hz
                ));
            }
            
            spectralOverlaps.emplace_back(std::make_tuple(
                std::move(spectralSegments),
                ov->lower_hz,
                ov->upper_hz
            ));
        }
        
        MaskOverlap maskOverlap = std::make_tuple(
            std::move(spectralOverlaps),
            out.lower_hz,
            out.upper_hz,
            out.total
        );
        
        emane_rs_spectral_mask_free_overlap(&out);
        return maskOverlap;
    }
    
    return MaskOverlap{};
}

std::uint64_t
EMANE::SpectralMaskManager::getPrimarySignalBandwidth(std::uint16_t u16SpectalMaskId) const
{
    return emane_rs_spectral_mask_get_primary_bandwidth(u16SpectalMaskId);
}
