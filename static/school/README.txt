School website static files
===========================

Contents
--------
- index.html   — portal UI
- school.js    — portal logic (Basic Auth via sessionStorage after login)
- login.html   — staff sign-in page

Deploy
------
Copy into lab-api static/school/ (or wherever ServeDir mounts /school/).

Nginx (required for login page):
- /school/ and /school/login.html — public static (no auth_basic)
- /school/api/ — auth_basic + proxy_set_header X-Authenticated-User $remote_user

Without that split, login.html cannot load (chicken-and-egg with Basic Auth).
