document.addEventListener("DOMContentLoaded", function () {
  const requestTokenForm = document.getElementById("requestTokenForm");
  const resetPasswordForm = document.getElementById("resetPasswordForm");

  // Request reset token
  requestTokenForm.addEventListener("submit", async function (e) {
    e.preventDefault();
    const email = e.target.querySelector('input[type="email"]').value.trim();

    if (!email) {
      alert("❌ Please enter your email.");
      return;
    }

    try {
      const result = await window.__TAURI__.core.invoke("tauri_forgot_password", {
        email: email
      });
      
      alert("✅ " + result);
    } catch (error) {
      console.error("Forgot password error:", error);
      alert("⚠️ " + error);
    }
  });

  // Reset password with token
  resetPasswordForm.addEventListener("submit", async function (e) {
    e.preventDefault();
    
    const email = e.target.querySelector('input[type="email"]').value.trim();
    const token = e.target.querySelector('input[name="reset_token"]').value.trim();
    const newPassword = e.target.querySelector('input[name="new_password"]').value.trim();
    const confirmPassword = e.target.querySelector('input[name="confirm_password"]').value.trim();

    if (!email || !token || !newPassword || !confirmPassword) {
      alert("❌ Please fill in all fields.");
      return;
    }

    if (newPassword !== confirmPassword) {
      alert("❌ Passwords do not match.");
      return;
    }

    if (newPassword.length < 6) {
      alert("❌ Password must be at least 6 characters long.");
      return;
    }

    try {
      const result = await window.__TAURI__.core.invoke("reset_password", {
        email: email,
        token: token,
        newPassword: newPassword
      });
      
      alert("✅ " + result);
      window.location.href = "login.html";
    } catch (error) {
      console.error("Reset password error:", error);
      alert("⚠️ " + error);
    }
  });
});