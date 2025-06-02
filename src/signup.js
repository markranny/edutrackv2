document.addEventListener("DOMContentLoaded", function () {
  const signupForm = document.getElementById("signupForm");
  const role = sessionStorage.getItem("selectedRole") || "student";

  signupForm.addEventListener("submit", async function (e) {
    e.preventDefault();

    const firstname = document.getElementById("firstname").value.trim();
    const lastname = document.getElementById("lastname").value.trim();
    const email = document.getElementById("email").value.trim();
    const password = document.getElementById("password").value.trim();
    const confirmPassword = document.getElementById("confirmPassword").value.trim();

    if (password !== confirmPassword) {
      alert("❌ Password and Confirm Password do not match.");
      return;
    }
    if (!firstname || !lastname || !email || !password) {
      alert("❌ Please fill all required fields.");
      return;
    }

    if (password.length < 6) {
      alert("❌ Password must be at least 6 characters long.");
      return;
    }

    try {
      // Call Tauri command instead of HTTP request
      const result = await window.__TAURI__.core.invoke("tauri_register", {
        payload: { 
          firstname, 
          lastname, 
          email, 
          password, 
          role 
        }
      });

      alert("✅ " + result);
      window.location.href = "login.html";
    } catch (error) {
      console.error("Signup error:", error);
      alert("⚠️ " + error);
    }
  });
});