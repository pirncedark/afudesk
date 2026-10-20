import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Sürüm imzası: CI key.properties dosyasını secret'lardan yazar. Yoksa debug anahtarı.
val anahtarDosyasi = rootProject.file("key.properties")
val anahtar = Properties().apply { if (anahtarDosyasi.exists()) anahtarDosyasi.inputStream().use { load(it) } }

android {
    buildFeatures { buildConfig = true }
    namespace = "com.afu.afudesk"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "com.afu.afudesk"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = maxOf(flutter.minSdkVersion, 23)
        // Açık istemci kimliği derleme yapılandırmasıdır; gerçek değer repoya girmez.
        val googleId = providers.gradleProperty("AFUDESK_GOOGLE_WEB_CLIENT_ID").orElse("").get()
        require(googleId.isEmpty() || googleId.matches(Regex("[A-Za-z0-9._-]+\\.apps\\.googleusercontent\\.com")))
        buildConfigField("String", "GOOGLE_WEB_CLIENT_ID", "\"$googleId\"")
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    signingConfigs {
        if (anahtarDosyasi.exists()) {
            create("afu") {
                storeFile = file(anahtar.getProperty("storeFile"))
                storePassword = anahtar.getProperty("storePassword")
                keyAlias = anahtar.getProperty("keyAlias")
                keyPassword = anahtar.getProperty("keyPassword")
                storeType = "pkcs12"
            }
        }
    }

    buildTypes {
        release {
            signingConfig = if (anahtarDosyasi.exists()) signingConfigs.getByName("afu") else signingConfigs.getByName("debug")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}

dependencies {
    implementation("androidx.credentials:credentials:1.6.0")
    implementation("androidx.credentials:credentials-play-services-auth:1.6.0")
    implementation("com.google.android.libraries.identity.googleid:googleid:1.1.1")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}
