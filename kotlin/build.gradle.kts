plugins {
    kotlin("jvm") version "1.9.0"
}

group = "com.edax"
version = "1.0.0"

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
}
