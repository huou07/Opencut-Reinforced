plugins { id("com.android.application") }
android {
    namespace = "dev.opencut.saffixture"
    compileSdk = 36
    defaultConfig {
        applicationId = "dev.opencut.saffixture"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1"
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    sourceSets.getByName("main").assets.srcDir(rootProject.projectDir.resolve("../../../crates/or_media/tests/fixtures"))
}
