self.addEventListener("push", (e) => {
	console.log("hi");
	e.waitUntil(self.registration.showNotification("test"));
});
