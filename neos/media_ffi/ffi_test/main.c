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

    /* ==================================================================
     * The video bridge — same checks, same shape, a real independent
     * verification of media_ffi_crystallise_video and friends.
     * ================================================================== */

    /* A real, quantisable-scale video: 20 frames of a 2x2 grid, amplitude
     * scaled the way _mkb/timecrystal.md §5.3 requires (2e-8), varying
     * frame to frame with a real sine so the crystallisation has genuine
     * structure to find. */
    {
        const size_t frame_count = 20, width = 2, height = 2;
        double frames[20 * 2 * 2];
        for (size_t i = 0; i < frame_count; i++) {
            double value = (1.0 + sin(i * 0.3)) * 2.0e-8;
            for (size_t p = 0; p < width * height; p++) {
                frames[i * width * height + p] = value;
            }
        }

        MediaFfiVideoResult *vresult = media_ffi_crystallise_video(frames, frame_count, width, height, 30.0, 3);
        CHECK(vresult != NULL, "a well-formed video returns a non-NULL handle");
        CHECK(media_ffi_video_result_is_ok(vresult) == 1, "the video handle reports success");
        CHECK(media_ffi_video_result_error_message(vresult) == NULL, "a successful video handle has no error message");

        size_t nodes = media_ffi_video_result_node_count(vresult);
        printf("  video: %zu phase-space node(s)\n", nodes);
        CHECK(nodes > 0, "a real varying video embeds at least one phase-space node");

        double energy = media_ffi_video_result_input_energy(vresult);
        CHECK(!isnan(energy) && energy > 0.0, "the video's input energy is a real, positive number");
        printf("  video: input energy %.6e J, conserving: %d, fundamental %.4f Hz\n",
               energy,
               media_ffi_video_result_is_energy_conserving(vresult),
               media_ffi_video_result_fundamental_hz(vresult));

        double out[4] = { -999.0, -999.0, -999.0, -999.0 };
        int ok = media_ffi_video_result_node(vresult, 0, out);
        CHECK(ok == 1, "reading node 0's components through the output pointer succeeds");
        printf("  video: node 0 = [%.6f, %.6f, %.6f, %.6f]\n", out[0], out[1], out[2], out[3]);

        double junk[4] = { -1.0, -1.0, -1.0, -1.0 };
        int oob = media_ffi_video_result_node(vresult, 9999, junk);
        CHECK(oob == 0, "an out-of-range node index is refused, not read out of bounds");
        CHECK(junk[0] == -1.0 && junk[1] == -1.0 && junk[2] == -1.0 && junk[3] == -1.0,
              "a refused video read leaves the output buffer untouched");

        media_ffi_video_result_free(vresult);
    }

    /* NULL frame buffer. */
    MediaFfiVideoResult *null_video = media_ffi_crystallise_video(NULL, 20, 2, 2, 30.0, 3);
    CHECK(null_video == NULL, "a NULL frame buffer returns NULL directly");

    /* A real crystallisation-level failure: real 8-bit-scale pixel values,
     * never rescaled, overflowing the quantisable ceiling. */
    {
        const size_t frame_count = 5, width = 2, height = 2;
        double bad_frames[5 * 2 * 2];
        for (size_t i = 0; i < frame_count * width * height; i++) {
            bad_frames[i] = 128.0;
        }
        MediaFfiVideoResult *err_video = media_ffi_crystallise_video(bad_frames, frame_count, width, height, 30.0, 2);
        CHECK(err_video != NULL, "an unrescaled video still returns a valid, freeable handle");
        CHECK(media_ffi_video_result_is_ok(err_video) == 0, "the unrescaled video handle correctly reports failure");
        const char *vmsg = media_ffi_video_result_error_message(err_video);
        CHECK(vmsg != NULL, "a failed video handle has a real, non-NULL error message");
        if (vmsg != NULL) {
            printf("  video error message: %s\n", vmsg);
        }
        media_ffi_video_result_free(err_video);
    }

    media_ffi_video_result_free(NULL);
    printf("ok:   freeing a NULL video handle does not crash\n");

    /* ==================================================================
     * The audio bridge — media_ffi_embed_audio and friends.
     * ================================================================== */
    {
        double signal[64];
        for (size_t i = 0; i < 64; i++) {
            signal[i] = sin(i * 0.4);
        }
        MediaFfiAudioResult *aresult = media_ffi_embed_audio(signal, 64, 3);
        CHECK(aresult != NULL, "a well-formed signal returns a non-NULL handle");
        CHECK(media_ffi_audio_result_is_ok(aresult) == 1, "the audio handle reports success");

        size_t anodes = media_ffi_audio_result_node_count(aresult);
        printf("  audio: %zu phase-space node(s)\n", anodes);
        CHECK(anodes > 0, "a real signal embeds at least one phase-space node");

        double aout[4] = { -999.0, -999.0, -999.0, -999.0 };
        int aok = media_ffi_audio_result_node(aresult, 0, aout);
        CHECK(aok == 1, "reading audio node 0's components succeeds");
        printf("  audio: node 0 = [%.6f, %.6f, %.6f, %.6f]\n", aout[0], aout[1], aout[2], aout[3]);

        double ajunk[4] = { -1.0, -1.0, -1.0, -1.0 };
        int aoob = media_ffi_audio_result_node(aresult, 9999, ajunk);
        CHECK(aoob == 0, "an out-of-range audio node index is refused");
        CHECK(ajunk[0] == -1.0, "a refused audio read leaves the output buffer untouched");

        media_ffi_audio_result_free(aresult);
    }

    MediaFfiAudioResult *null_audio = media_ffi_embed_audio(NULL, 64, 3);
    CHECK(null_audio == NULL, "a NULL signal pointer returns NULL directly");

    {
        double short_signal[2] = { 1.0, 2.0 };
        MediaFfiAudioResult *err_audio = media_ffi_embed_audio(short_signal, 2, 10);
        CHECK(err_audio != NULL, "a too-short signal still returns a valid, freeable handle");
        CHECK(media_ffi_audio_result_is_ok(err_audio) == 0, "the too-short signal handle correctly reports failure");
        const char *amsg = media_ffi_audio_result_error_message(err_audio);
        CHECK(amsg != NULL, "a failed audio handle has a real, non-NULL error message");
        if (amsg != NULL) {
            printf("  audio error message: %s\n", amsg);
        }
        media_ffi_audio_result_free(err_audio);
    }

    media_ffi_audio_result_free(NULL);
    printf("ok:   freeing a NULL audio handle does not crash\n");

    /* ==================================================================
     * The text bridge — media_ffi_crystallise_text and friends.
     * ================================================================== */
    {
        const char *text = "first\nsecond line\nthird";
        MediaFfiTextResult *tresult = media_ffi_crystallise_text((const unsigned char *)text, strlen(text));
        CHECK(tresult != NULL, "a well-formed document returns a non-NULL handle");
        CHECK(media_ffi_text_result_is_ok(tresult) == 1, "the text handle reports success");

        size_t tnodes = media_ffi_text_result_node_count(tresult);
        size_t bifurcations = media_ffi_text_result_bifurcations(tresult);
        double extent = media_ffi_text_result_extent(tresult);
        printf("  text: %zu node(s), %zu bifurcation(s), extent %.5f\n", tnodes, bifurcations, extent);
        CHECK(tnodes == 21, "21 non-newline characters in this document");
        CHECK(bifurcations == 2, "two real line breaks");
        CHECK(!isnan(extent) && extent > 1.0, "extent grows past 1.0 after real bifurcations");

        unsigned int codepoint = 0;
        double phase = 0.0;
        int tok = media_ffi_text_result_node(tresult, 0, &codepoint, &phase);
        CHECK(tok == 1, "reading text node 0 succeeds");
        CHECK(codepoint == (unsigned int)'f', "node 0 is the document's first real character, 'f'");
        printf("  text: node 0 codepoint=%u ('%c'), phase=%.4f\n", codepoint, (char)codepoint, phase);

        unsigned int junk_cp = 999;
        double junk_ph = -999.0;
        int toob = media_ffi_text_result_node(tresult, 9999, &junk_cp, &junk_ph);
        CHECK(toob == 0, "an out-of-range text node index is refused");
        CHECK(junk_cp == 999 && junk_ph == -999.0, "a refused text read leaves the output untouched");

        media_ffi_text_result_free(tresult);
    }

    MediaFfiTextResult *null_text = media_ffi_crystallise_text(NULL, 10);
    CHECK(null_text == NULL, "a NULL text pointer returns NULL directly");

    {
        /* Five real line breaks — over Crystal::max_bifurcations()'s
         * ceiling of 3. */
        const char *deep = "a\na\na\na\na\n";
        MediaFfiTextResult *err_text = media_ffi_crystallise_text((const unsigned char *)deep, strlen(deep));
        CHECK(err_text != NULL, "an over-deep document still returns a valid, freeable handle");
        CHECK(media_ffi_text_result_is_ok(err_text) == 0, "the over-deep document handle correctly reports failure");
        const char *tmsg = media_ffi_text_result_error_message(err_text);
        CHECK(tmsg != NULL, "a failed text handle has a real, non-NULL error message");
        if (tmsg != NULL) {
            printf("  text error message: %s\n", tmsg);
        }
        media_ffi_text_result_free(err_text);
    }

    {
        /* A bare UTF-8 continuation byte: invalid on its own. */
        const unsigned char bad_utf8[] = { 'a', 0x80, 'b' };
        MediaFfiTextResult *err_utf8 = media_ffi_crystallise_text(bad_utf8, 3);
        CHECK(err_utf8 != NULL, "invalid UTF-8 still returns a valid, freeable handle");
        CHECK(media_ffi_text_result_is_ok(err_utf8) == 0, "invalid UTF-8 correctly reports failure, not a crash");
        media_ffi_text_result_free(err_utf8);
    }

    media_ffi_text_result_free(NULL);
    printf("ok:   freeing a NULL text handle does not crash\n");

    if (failures == 0) {
        printf("\nALL CHECKS PASSED\n");
        return 0;
    } else {
        printf("\n%d CHECK(S) FAILED\n", failures);
        return 1;
    }
}
