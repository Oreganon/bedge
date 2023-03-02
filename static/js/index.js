function urlBase64ToUint8Array(base64String) {
    var padding = '='.repeat((4 - base64String.length % 4) % 4);
    var base64 = (base64String + padding)
        .replace(/\-/g, '+')
        .replace(/_/g, '/');

    var rawData = window.atob(base64);
    var outputArray = new Uint8Array(rawData.length);

    for (var i = 0; i < rawData.length; ++i) {
        outputArray[i] = rawData.charCodeAt(i);
    }
    return outputArray;
}

let public_key = "BIagy15bBenUFOodCAyj_X4Xb_6T0vKCzAso9WYSHltUXU5CHqLH1kb-knfjwdDjOwTSGz5Ywr1t_atr-Lb9-Bk=";

function subscribeUserToPush() {
  return navigator.serviceWorker
    .register('/sw.js')
    .then(function (registration) {
      const subscribeOptions = {
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(public_key),
      };

      return registration.pushManager.subscribe(subscribeOptions);
    })
    .then(function (pushSubscription) {
      console.log(
        'Received PushSubscription: ',
        JSON.stringify(pushSubscription),
      );
      return pushSubscription;
    }).catch(console.error);
}

function sendSubscriptionToBackEnd(subscription) {
  let body = JSON.stringify(subscription);
  console.log(body);
  return fetch('/save-subscription/', {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
    },
    body: body,
  })
    .then(function (response) {
      if (!response.ok) {
        throw new Error('Bad status code from server.');
      }

      return response.json();
    })
    .then(function (responseData) {
      if (!(responseData.data && responseData.data.success)) {
        throw new Error('Bad response from server.');
      }
    });
}

function subscribe() {
  if ('serviceWorker' in navigator) {
    let subscription = subscribeUserToPush().then(subscription => {
      console.log(subscription);
      sendSubscriptionToBackEnd(subscription);
    });
  } else {
    alert("Scuffed browser...tag me");
  }
}

function notifyMe() {
  if (!("Notification" in window)) {
    // Check if the browser supports notifications
    alert("This browser does not support desktop notification");
  } else if (Notification.permission === "granted") {
    subscribe();
  } else if (Notification.permission !== "denied") {
    // We need to ask the user for permission
    Notification.requestPermission().then((permission) => {
      // If the user accepts, let's create a notification
      if (permission === "granted") {
        subscribe();
      } else {
	alert("then not...");
      }
    });
  }
}


