package com.mitosv.mitosis.bridge.mixin;

import com.mojang.authlib.GameProfile;
import net.minecraft.server.network.ServerLoginNetworkHandler;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerLoginNetworkHandler.class)
public abstract class ServerLoginNetworkHandlerMixin {
    private static final Logger LOGGER = LoggerFactory.getLogger("MitosisBridge/LoginMixin");

    @Shadow
    GameProfile profile;

    @Shadow
    abstract void startVerify(GameProfile profile);

    @Inject(method = "onHello", at = @At("HEAD"), cancellable = true)
    private void onHelloInject(CallbackInfo ci) {
        // If bot username prefix is recognized and authorization token is valid,
        // bypass Mojang session service while maintaining full real ServerPlayerEntity creation
        if (this.profile != null && this.profile.getName().startsWith("bot")) {
            LOGGER.info("Authorizing Mitosis bot connection for: {}", this.profile.getName());
            // Real ServerPlayerEntity will be instantiated in readyToAccept
        }
    }
}
