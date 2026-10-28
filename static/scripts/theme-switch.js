"use strict";

// val: "light" | "dark"
function renderTheme(val) {
    document.documentElement.setAttribute("data-theme", val);
    document.getElementById("syntax-theme").href = `/giallo-${val}.css`
}

function changeThemeTo(val) {
    if (val === "system") {
        setThemeToSystemPref();
        // delete explicit theme pref from browser storage
        if (storageAvailable("localStorage")) {
            localStorage.removeItem("lang-research-theme");
        }
    } else {
        renderTheme(val);
        // save theme prefs in the browser
        if (storageAvailable("localStorage")) {
            localStorage.setItem("lang-research-theme", val);
        }
    }
    // the theme dropdown will close itself when returning to the dropdown() caller
}

function dropdown () {
    document.getElementById("theme-choice").classList.toggle("showThemeDropdown");
}

// courtesy MDN
function storageAvailable(type) {
    let storage;
    try {
        storage = window[type];
        const x = "__storage_test__";
        storage.setItem(x, x);
        storage.removeItem(x);
        return true;
    } catch (e) {
        return (
            e instanceof DOMException &&
                e.name === "QuotaExceededError" &&
                // acknowledge QuotaExceededError only if there's something already stored
            storage &&
                storage.length !== 0
        );
    }
}

function handleBlur(event) {
    const parent = document.getElementById("theme-choice");
    if (!parent.contains(document.activeElement) &&
        !parent.contains(event.relatedTarget) &&
        parent.classList.contains("showThemeDropdown")
       ) {
        console.debug('Closing the dropdown');
        parent.classList.remove("showThemeDropdown");
    }
}

function setThemeToSystemPref() {
    if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
        renderTheme("dark");
    } else {
        renderTheme("light");
    }
}

// Check for saved user preference on load, else check and save user agent prefs
let savedTheme = null;
if (storageAvailable("localStorage")) {
    savedTheme = localStorage.getItem("lang-research-theme");
}
if (savedTheme) {
    renderTheme(savedTheme);
} else {
    setThemeToSystemPref();
}
