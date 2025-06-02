document.addEventListener("DOMContentLoaded", function () {
  const loginForm = document.getElementById("loginForm");
  const role = sessionStorage.getItem("selectedRole");

  loginForm.addEventListener("submit", async function (e) {
    e.preventDefault();
    const email = document.querySelector('input[type="email"]').value.trim();
    const password = document.querySelector('input[type="password"]').value.trim();

    if (!email || !password) {
      alert("❌ Please fill in all fields.");
      return;
    }

    if (!role) {
      alert("❌ Please select a role first.");
      window.location.href = "index.html";
      return;
    }

    try {
      // Call Tauri command instead of HTTP request
      const result = await window.__TAURI__.invoke("tauri_login", {
        payload: { email, password, role }
      });

      if (result.success) {
        alert(`✅ Welcome!`);
        // Store user info in sessionStorage
        sessionStorage.setItem("currentUser", JSON.stringify(result.user));
        
        // Redirect based on role
        if (role === "student") {
          window.location.href = "marking.html";
        } else {
          window.location.href = "dash.html";
        }
      } else {
        alert("❌ " + result.message);
      }
    } catch (error) {
      console.error("Login error:", error);
      alert("❌ Login failed: " + error);
    }
  });
});