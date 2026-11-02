package com.mitosv.mitosis.bridge.auth;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.UUID;

public class MitosisAuthVerifier {
    private static final Logger LOGGER = LoggerFactory.getLogger("MitosisBridge/Auth");
    private static String secretKey = "mitosis_super_secure_installation_key_change_me";

    public static void setSecretKey(String key) {
        secretKey = key;
    }

    public static boolean verifyToken(String tokenB64, UUID expectedUuid, String expectedUsername) {
        try {
            byte[] decoded = Base64.getDecoder().decode(tokenB64);
            String tokenStr = new String(decoded, StandardCharsets.UTF_8);
            String[] parts = tokenStr.split(":");
            if (parts.length != 4) {
                LOGGER.warn("Malformed token structure: {}", tokenStr);
                return false;
            }

            long expiresAtMs = Long.parseLong(parts[0]);
            UUID tokenUuid = UUID.fromString(parts[1]);
            String tokenUsername = parts[2];
            String providedSig = parts[3];

            long nowMs = System.currentTimeMillis();
            if (nowMs > expiresAtMs) {
                LOGGER.warn("Token expired for bot {}: now={}, expires={}", tokenUsername, nowMs, expiresAtMs);
                return false;
            }

            if (!tokenUuid.equals(expectedUuid) || !tokenUsername.equals(expectedUsername)) {
                LOGGER.warn("Token identity mismatch: expected {}/{} but got {}/{}", expectedUsername, expectedUuid, tokenUsername, tokenUuid);
                return false;
            }

            String payload = parts[0] + ":" + parts[1] + ":" + parts[2];
            Mac mac = Mac.getInstance("HmacSHA256");
            mac.init(new SecretKeySpec(secretKey.getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
            byte[] expectedHmac = mac.doFinal(payload.getBytes(StandardCharsets.UTF_8));

            StringBuilder sb = new StringBuilder();
            for (byte b : expectedHmac) {
                sb.append(String.format("%02x", b));
            }
            String expectedSig = sb.toString();

            return providedSig.equalsIgnoreCase(expectedSig);
        } catch (Exception e) {
            LOGGER.error("Auth verification failed: {}", e.getMessage());
            return false;
        }
    }
}
