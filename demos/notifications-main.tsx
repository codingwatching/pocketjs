// @title psp-ui: Notifications
import Notifications, { notificationsFrame } from "./notifications.tsx";
import { mount } from "../src/index.ts";

mount(() => <Notifications />, { beforeFrame: notificationsFrame });
