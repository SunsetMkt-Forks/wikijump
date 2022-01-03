import { readable, type Subscriber } from "svelte/store"
import {
  Api,
  ContentType,
  type UserIdentity,
  type UserInfo,
  type UserProfile
} from "../vendor/api"

const API_PATH = "/api--v0"

class WikijumpAPIInstance extends Api<void> {
  // private properties have _ as a prefix to prevent conflicting with any
  // autogenerated API methods

  /** Current CSRF token. */
  private declare _CSRF?: string

  /** @param headers - Extra headers to send with every request. */
  constructor(headers: Record<string, string> = {}) {
    super({
      baseUrl: API_PATH,
      baseApiParams: {
        headers: {
          "Accept": "application/json",
          "Content-Type": "application/json",
          ...headers
        },
        secure: true,
        format: "json"
      },
      // this gets ran on every request,
      // so this is more for setting up an API request
      // than just handling security
      securityWorker: () => ({ headers: this.getSecurityHeaders() })
    })

    this._hijackAuthMethods()

    // update authentication status, as we may already be logged in
    this.authCheck().catch()
  }

  private _hijackAuthMethods() {
    // authLogin and authRefresh are special in that they regenerate your session.
    // this invalidates your old CSRF token, so we need to update it,
    // which means overriding the old methods with new ones.

    // additionally, we want to update the authed store to whatever
    // our authentication status is.
    // so, we need to hijack all the auth methods

    // unfortunately we can't use super.function because
    // the auto-generated "method" is actually a value and not a method.

    const login = this.authLogin.bind(this)
    const logout = this.authLogout.bind(this)
    const refresh = this.authRefresh.bind(this)
    const check = this.authCheck.bind(this)

    // this call automatically logs you in, so this needs to be bound as well
    const register = this.accountRegister.bind(this)

    // we don't actually need to hijack authConfirm
    // const confirm = this.authConfirm.bind(this)

    this.authLogin = async (data, requestParams) => {
      const res = await login(data, requestParams)
      this._CSRF = res.csrf
      authSet(true)
      this.updateIdentity()
      return res
    }

    this.authLogout = async requestParams => {
      await logout(requestParams)
      authSet(false)
      this.updateIdentity()
    }

    this.authRefresh = async requestParams => {
      const res = await refresh(requestParams)
      this._CSRF = res.csrf
      this.updateIdentity()
      return res
    }

    this.authCheck = async requestParams => {
      const res = await check(requestParams)
      authSet(res.authed)
      this.updateIdentity()
      return res
    }

    this.accountRegister = async (data, requestParams) => {
      const res = await register(data, requestParams)
      this._CSRF = res.csrf
      authSet(true)
      this.updateIdentity()
      return res
    }
  }

  /**
   * Updates the current client `UserIdentity`. Usually called when
   * authentication state changes.
   */
  private async updateIdentity() {
    if (isAuthenticated()) {
      try {
        const newIdentity = await this.userClientGet()
        // avoid updating the identity if it hasn't actually changed
        if (currentIdentity()?.username !== newIdentity.username) {
          identitySet(newIdentity)
        }
      } catch (err) {
        console.error(err)
        identitySet(null)
      }
    } else {
      identitySet(null)
    }
  }

  /**
   * Helper for sending a `GET` request via the API.
   *
   * @param to - The path to send the request to.
   * @param query - The query parameters to send, if any.
   */
  get<T = void>(to: string, query: Record<string, string> = {}) {
    const url = new URL(to)
    const baseUrl = url.origin
    const path = url.pathname
    return this.request<T, void>({ method: "GET", baseUrl, path, query })
  }

  /**
   * Helper for sending a `POST` request via the API.
   *
   * @param to - The path to send the request to.
   * @param body - The data to send, if any.
   */
  post<T = void>(to: string, body: any = {}) {
    const url = new URL(to)
    const baseUrl = url.origin
    const path = url.pathname
    const type = ContentType.Json
    return this.request<T, void>({ method: "POST", baseUrl, path, body, type })
  }

  /**
   * Executes an API request and catches any errors by returning `null`.
   * This can be used when the result of a request is easily discarded.
   *
   * @param method - The API method to call.
   * @param args - The arguments to pass to the method.
   */
  async try<
    M extends keyof FilterFor<Api<void>, AnyFunction>,
    F extends AnyFunction = FilterFor<Api<void>, AnyFunction>[M]
  >(method: M, ...args: Parameters<F>): Promise<null | Awaited<ReturnType<F>>> {
    try {
      // @ts-ignore - TS can't quite figure this one out
      return await this[method](...args)
    } catch {
      return null
    }
  }

  /** Gets the current security headers. */
  getSecurityHeaders():
    | { "X-CSRF-TOKEN": string }
    | { "X-CSRF-TOKEN": string; "X-XSRF-TOKEN": string } {
    const csrf = this._CSRF ?? getCSRFMeta()
    const xsrf = getCSRFCookie()
    const securityHeaders = xsrf
      ? { "X-CSRF-TOKEN": csrf, "X-XSRF-TOKEN": xsrf }
      : { "X-CSRF-TOKEN": csrf }

    return securityHeaders
  }

  /**
   * Attempts to return the given query parameter from the current URL.
   *
   * @param name - The name of the query parameter to return.
   */
  getQueryParameter(key: string) {
    return new URLSearchParams(window.location.search).get(key)
  }

  /**
   * Attempts to get the specified path segment (via index) from the current URL.
   *
   * @param index - The index of the path segment to return.
   */
  getPathSegment(index: number): string | null {
    return window.location.pathname.split("/")[index + 1] ?? null
  }

  /**
   * Returns a base URL but for a different subdomain.
   *
   * @param subdomain - The subdomain to use.
   */
  subdomainURL(subdomain: string) {
    return `${window.location.protocol}//${subdomain}.${window.location.host}/${API_PATH}`
  }

  /**
   * Gets a user via their ID or slug. If the user is not found, returns `null`.
   *
   * @param user - The user to get.
   * @param detail - The detail level to get.
   */
  async getUser(user: number | string): Promise<null | UserIdentity>
  async getUser(user: number | string, detail: "identity"): Promise<null | UserIdentity>
  async getUser(user: number | string, detail: "info"): Promise<null | UserInfo>
  async getUser(user: number | string, detail: "profile"): Promise<null | UserProfile>
  async getUser(
    user: number | string,
    detail: "identity" | "info" | "profile" = "identity"
  ) {
    return this.try("userGet", typeof user === "number" ? "id" : "slug", user, { detail })
  }

  /**
   * Gets the current user's identity. If the user is not logged in, returns `null`.
   *
   * @param detail - The detail level to get.
   */
  async getClient(): Promise<null | UserIdentity>
  async getClient(detail: "identity"): Promise<null | UserIdentity>
  async getClient(detail: "info"): Promise<null | UserInfo>
  async getClient(detail: "profile"): Promise<null | UserProfile>
  async getClient(detail: "identity" | "info" | "profile" = "identity") {
    return this.try("userClientGet", { detail })
  }
}

let authSet: Subscriber<boolean>
let identitySet: Subscriber<null | UserIdentity>

/** Readable store holding the current authentication state. */
export const authed = readable<boolean>(false, set => void (authSet = set))

/** Readable store holding the current client `UserIdentity`. */
export const identity = readable<null | UserIdentity>(
  null,
  set => void (identitySet = set)
)

let isAuthedBinding = false
let identityBinding: null | UserIdentity = null

authed.subscribe(state => void (isAuthedBinding = state))
identity.subscribe(identity => void (identityBinding = identity))

/** Returns the current authentication state. */
export function isAuthenticated() {
  return isAuthedBinding
}

/** Returns the current `UserIdentity` for the client. */
export function currentIdentity() {
  return identityBinding
}

/** Wikijump API. */
export const WikijumpAPI = new WikijumpAPIInstance()

/**
 * Retrieves the CSRF token from the `<meta name="csrf-token" ...>` tag in
 * the `<head>`. This should always be present, so this function throws if
 * that element can't be found.
 */
function getCSRFMeta() {
  const meta = document.head.querySelector("meta[name=csrf-token]")
  if (!meta) throw new Error("No CSRF meta tag found")
  return meta.getAttribute("content")!
}

/** Retrieves the CSRF token from the `XSRF-TOKEN` cookie, if it exists. */
function getCSRFCookie() {
  const value = document.cookie
    .split(/;\s*/)
    .find(c => c.startsWith("XSRF-TOKEN="))
    ?.split("=")[1]
  return value
}
