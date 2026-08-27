/*
 * A real, independent C program linked against the compiled media_ffi
 * library — not Rust calling its own extern "C" functions, which would
 * prove nothing about actual cross-language ABI safety. This is the
 * genuine verification: does a real C compiler, given only media_ffi.h and
 * the built .lib/.dll, actually produce a working program?
 *
 * Exit code 0 means every check below passed. Any failure prints what went
 * wrong and exits non-zero, so this can gate a build the same way this
 * workspace's Rust tests already do.
 */

#include <stdio.h>
#include <string.h>
#include <math.h>
#include "../media_ffi.h"

static int failures = 0;

#define CHECK(cond, msg) \
    do { \
        if (!(cond)) { \
            printf("FAIL: %s\n", msg); \
            failures++; \
        } else { \
            printf("ok:   %s\n", msg); \
        } \
    } while (0)

int main(void) {
    /* The identical embedded fixture neos/src/main.rs's own embedded_ppm()
     * uses: a real 4x4 P5 (greyscale) PPM, 16 pixels, dividing evenly
     * across the Tetryen's four faces. */
    const unsigned char ppm[] =
        "P5\n4 4\n255\n"
        "\x01\x02\x03\x04\x05\x06\x07\x08\x09\x01\x02\x03\x04\x05\x06\x07";
    size_t ppm_len = sizeof(ppm) - 1; /* drop the trailing NUL the string literal adds */

    /* ---- the real success path ---------------------------------------- */
    MediaFfiImageResult *result = media_ffi_crystallise_image(ppm, ppm_len);
    CHECK(result != NULL, "a well-formed embedded PPM returns a non-NULL handle");

    CHECK(media_ffi_image_result_is_ok(result) == 1, "the handle reports success");
    CHECK(media_ffi_image_result_error_message(result) == NULL, "a successful handle has no error message");

    size_t faces = media_ffi_image_result_face_count(result);
    CHECK(faces == 4, "a Tetryen projection always has exactly 4 faces");

    printf("  face energies: ");
    double total_energy = 0.0;
    for (size_t f = 0; f < faces; f++) {
        double e = media_ffi_image_result_face_energy(result, f);
        CHECK(!isnan(e), "a valid face's energy is a real number, not NAN");
        CHECK(e >= 0.0, "energy (a sum of squared magnitudes) is never negative");
        total_energy += e;
        printf("%.6f ", e);
    }
    printf("\n");
    CHECK(total_energy > 0.0, "a non-trivial image has nonzero total frequency-side energy");

    size_t coeffs0 = media_ffi_image_result_coefficient_count(result, 0);
    CHECK(coeffs0 > 0, "face 0 has at least one frequency coefficient");

    double re = 0.0, im = 0.0;
    int ok = media_ffi_image_result_coefficient(result, 0, 0, &re, &im);
    CHECK(ok == 1, "reading face 0's first coefficient through the output pointers succeeds");
    printf("  face 0, coefficient 0: %.6f + %.6fi\n", re, im);

    /* Out-of-range face/index must fail cleanly, not crash. */
    double junk_re = -999.0, junk_im = -999.0;
    int oob = media_ffi_image_result_coefficient(result, 99, 0, &junk_re, &junk_im);
    CHECK(oob == 0, "an out-of-range face index is refused, not read out of bounds");
    CHECK(junk_re == -999.0 && junk_im == -999.0, "a refused read leaves the output pointers untouched");

    media_ffi_image_result_free(result);

    /* ---- the null-input path ------------------------------------------- */
    MediaFfiImageResult *null_result = media_ffi_crystallise_image(NULL, 100);
    CHECK(null_result == NULL, "a NULL byte pointer returns NULL directly, nothing to allocate a handle for");

    /* ---- the real crystallisation-error path ---------------------------
     * Not garbage bytes (decode_ppm would just reject those the same way);
     * a well-formed PPM header whose declared pixel count cannot decode
     * cleanly - specifically, one byte short of what its own header
     * promises, which is a real, honest malformed-file case. */
    const unsigned char bad_ppm[] = "P5\n4 4\n255\n\x01\x02\x03";
    size_t bad_len = sizeof(bad_ppm) - 1;
    MediaFfiImageResult *err_result = media_ffi_crystallise_image(bad_ppm, bad_len);
    CHECK(err_result != NULL, "a decode failure still returns a valid, freeable handle");
    CHECK(media_ffi_image_result_is_ok(err_result) == 0, "the handle correctly reports failure");
    const char *msg = media_ffi_image_result_error_message(err_result);
    CHECK(msg != NULL, "a failed handle has a real, non-NULL error message");
    if (msg != NULL) {
        printf("  error message: %s\n", msg);
        CHECK(strlen(msg) > 0, "the error message is not an empty string");
    }
    CHECK(media_ffi_image_result_face_count(err_result) == 0, "a failed handle reports zero faces");

    media_ffi_image_result_free(err_result);

    /* free(NULL) must be a safe no-op, matching C's own convention. */
    media_ffi_image_result_free(NULL);
    printf("ok:   freeing a NULL handle does not crash\n");

    if (failures == 0) {
        printf("\nALL CHECKS PASSED\n");
        return 0;
    } else {
        printf("\n%d CHECK(S) FAILED\n", failures);
        return 1;
    }
}
