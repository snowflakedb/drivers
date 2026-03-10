import * as $protobuf from "protobufjs";
import Long = require("long");
/** Namespace database_driver_v1. */
export namespace database_driver_v1 {

    /** StatusCode enum. */
    enum StatusCode {
        STATUS_CODE_UNSPECIFIED = 0,
        STATUS_CODE_OK = 1,
        STATUS_CODE_AUTHENTICATION_ERROR = 2,
        STATUS_CODE_NOT_IMPLEMENTED = 3,
        STATUS_CODE_NOT_FOUND = 4,
        STATUS_CODE_ALREADY_EXISTS = 5,
        STATUS_CODE_INVALID_ARGUMENT = 6,
        STATUS_CODE_INVALID_STATE = 7,
        STATUS_CODE_INVALID_DATA = 8,
        STATUS_CODE_IO = 9,
        STATUS_CODE_CANCELLED = 10,
        STATUS_CODE_UNAUTHENTICATED = 11,
        STATUS_CODE_UNAUTHORIZED = 12,
        STATUS_CODE_GENERIC_ERROR = 13,
        STATUS_CODE_INTERNAL_ERROR = 14,
        STATUS_CODE_MISSING_PARAMETER = 15,
        STATUS_CODE_INVALID_PARAMETER_VALUE = 16,
        STATUS_CODE_LOGIN_ERROR = 17
    }

    /** InfoCode enum. */
    enum InfoCode {
        INFO_CODE_UNSPECIFIED = 0,
        INFO_CODE_VENDOR_NAME = 1,
        INFO_CODE_VENDOR_VERSION = 2,
        INFO_CODE_VENDOR_ARROW_VERSION = 3,
        INFO_CODE_VENDOR_SQL = 101,
        INFO_CODE_VENDOR_SUBSTRAIT = 102,
        INFO_CODE_VENDOR_SUBSTRAIT_MIN_VERSION = 103,
        INFO_CODE_VENDOR_SUBSTRAIT_MAX_VERSION = 104,
        INFO_CODE_DRIVER_NAME = 201,
        INFO_CODE_DRIVER_VERSION = 202,
        INFO_CODE_DRIVER_ARROW_VERSION = 203,
        INFO_CODE_DRIVER_ADBC_VERSION = 204
    }

    /** Properties of an ErrorDetail. */
    interface IErrorDetail {

        /** ErrorDetail key */
        key?: (string|null);

        /** ErrorDetail value */
        value?: (string|null);
    }

    /** Represents an ErrorDetail. */
    class ErrorDetail implements IErrorDetail {

        /**
         * Constructs a new ErrorDetail.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IErrorDetail);

        /** ErrorDetail key. */
        public key: string;

        /** ErrorDetail value. */
        public value: string;

        /**
         * Creates a new ErrorDetail instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ErrorDetail instance
         */
        public static create(properties?: database_driver_v1.IErrorDetail): database_driver_v1.ErrorDetail;

        /**
         * Encodes the specified ErrorDetail message. Does not implicitly {@link database_driver_v1.ErrorDetail.verify|verify} messages.
         * @param message ErrorDetail message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IErrorDetail, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ErrorDetail message, length delimited. Does not implicitly {@link database_driver_v1.ErrorDetail.verify|verify} messages.
         * @param message ErrorDetail message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IErrorDetail, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ErrorDetail message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ErrorDetail
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ErrorDetail;

        /**
         * Decodes an ErrorDetail message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ErrorDetail
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ErrorDetail;

        /**
         * Verifies an ErrorDetail message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ErrorDetail message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ErrorDetail
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ErrorDetail;

        /**
         * Creates a plain object from an ErrorDetail message. Also converts values to other types if specified.
         * @param message ErrorDetail
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ErrorDetail, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ErrorDetail to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ErrorDetail
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an AuthenticationError. */
    interface IAuthenticationError {

        /** AuthenticationError detail */
        detail?: (string|null);
    }

    /** Represents an AuthenticationError. */
    class AuthenticationError implements IAuthenticationError {

        /**
         * Constructs a new AuthenticationError.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IAuthenticationError);

        /** AuthenticationError detail. */
        public detail: string;

        /**
         * Creates a new AuthenticationError instance using the specified properties.
         * @param [properties] Properties to set
         * @returns AuthenticationError instance
         */
        public static create(properties?: database_driver_v1.IAuthenticationError): database_driver_v1.AuthenticationError;

        /**
         * Encodes the specified AuthenticationError message. Does not implicitly {@link database_driver_v1.AuthenticationError.verify|verify} messages.
         * @param message AuthenticationError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IAuthenticationError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified AuthenticationError message, length delimited. Does not implicitly {@link database_driver_v1.AuthenticationError.verify|verify} messages.
         * @param message AuthenticationError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IAuthenticationError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an AuthenticationError message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns AuthenticationError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.AuthenticationError;

        /**
         * Decodes an AuthenticationError message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns AuthenticationError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.AuthenticationError;

        /**
         * Verifies an AuthenticationError message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an AuthenticationError message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns AuthenticationError
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.AuthenticationError;

        /**
         * Creates a plain object from an AuthenticationError message. Also converts values to other types if specified.
         * @param message AuthenticationError
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.AuthenticationError, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this AuthenticationError to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for AuthenticationError
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a GenericError. */
    interface IGenericError {
    }

    /** Represents a GenericError. */
    class GenericError implements IGenericError {

        /**
         * Constructs a new GenericError.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IGenericError);

        /**
         * Creates a new GenericError instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GenericError instance
         */
        public static create(properties?: database_driver_v1.IGenericError): database_driver_v1.GenericError;

        /**
         * Encodes the specified GenericError message. Does not implicitly {@link database_driver_v1.GenericError.verify|verify} messages.
         * @param message GenericError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IGenericError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GenericError message, length delimited. Does not implicitly {@link database_driver_v1.GenericError.verify|verify} messages.
         * @param message GenericError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IGenericError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GenericError message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GenericError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.GenericError;

        /**
         * Decodes a GenericError message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GenericError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.GenericError;

        /**
         * Verifies a GenericError message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GenericError message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GenericError
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.GenericError;

        /**
         * Creates a plain object from a GenericError message. Also converts values to other types if specified.
         * @param message GenericError
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.GenericError, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GenericError to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for GenericError
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an InternalError. */
    interface IInternalError {
    }

    /** Represents an InternalError. */
    class InternalError implements IInternalError {

        /**
         * Constructs a new InternalError.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IInternalError);

        /**
         * Creates a new InternalError instance using the specified properties.
         * @param [properties] Properties to set
         * @returns InternalError instance
         */
        public static create(properties?: database_driver_v1.IInternalError): database_driver_v1.InternalError;

        /**
         * Encodes the specified InternalError message. Does not implicitly {@link database_driver_v1.InternalError.verify|verify} messages.
         * @param message InternalError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IInternalError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified InternalError message, length delimited. Does not implicitly {@link database_driver_v1.InternalError.verify|verify} messages.
         * @param message InternalError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IInternalError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an InternalError message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns InternalError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.InternalError;

        /**
         * Decodes an InternalError message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns InternalError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.InternalError;

        /**
         * Verifies an InternalError message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an InternalError message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns InternalError
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.InternalError;

        /**
         * Creates a plain object from an InternalError message. Also converts values to other types if specified.
         * @param message InternalError
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.InternalError, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this InternalError to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for InternalError
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a LoginError. */
    interface ILoginError {

        /** LoginError message */
        message?: (string|null);

        /** LoginError code */
        code?: (number|null);
    }

    /** Represents a LoginError. */
    class LoginError implements ILoginError {

        /**
         * Constructs a new LoginError.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.ILoginError);

        /** LoginError message. */
        public message: string;

        /** LoginError code. */
        public code: number;

        /**
         * Creates a new LoginError instance using the specified properties.
         * @param [properties] Properties to set
         * @returns LoginError instance
         */
        public static create(properties?: database_driver_v1.ILoginError): database_driver_v1.LoginError;

        /**
         * Encodes the specified LoginError message. Does not implicitly {@link database_driver_v1.LoginError.verify|verify} messages.
         * @param message LoginError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.ILoginError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified LoginError message, length delimited. Does not implicitly {@link database_driver_v1.LoginError.verify|verify} messages.
         * @param message LoginError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.ILoginError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a LoginError message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns LoginError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.LoginError;

        /**
         * Decodes a LoginError message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns LoginError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.LoginError;

        /**
         * Verifies a LoginError message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a LoginError message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns LoginError
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.LoginError;

        /**
         * Creates a plain object from a LoginError message. Also converts values to other types if specified.
         * @param message LoginError
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.LoginError, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this LoginError to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for LoginError
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a MissingParameter. */
    interface IMissingParameter {

        /** MissingParameter parameter */
        parameter?: (string|null);
    }

    /** Represents a MissingParameter. */
    class MissingParameter implements IMissingParameter {

        /**
         * Constructs a new MissingParameter.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IMissingParameter);

        /** MissingParameter parameter. */
        public parameter: string;

        /**
         * Creates a new MissingParameter instance using the specified properties.
         * @param [properties] Properties to set
         * @returns MissingParameter instance
         */
        public static create(properties?: database_driver_v1.IMissingParameter): database_driver_v1.MissingParameter;

        /**
         * Encodes the specified MissingParameter message. Does not implicitly {@link database_driver_v1.MissingParameter.verify|verify} messages.
         * @param message MissingParameter message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IMissingParameter, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified MissingParameter message, length delimited. Does not implicitly {@link database_driver_v1.MissingParameter.verify|verify} messages.
         * @param message MissingParameter message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IMissingParameter, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a MissingParameter message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns MissingParameter
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.MissingParameter;

        /**
         * Decodes a MissingParameter message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns MissingParameter
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.MissingParameter;

        /**
         * Verifies a MissingParameter message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a MissingParameter message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns MissingParameter
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.MissingParameter;

        /**
         * Creates a plain object from a MissingParameter message. Also converts values to other types if specified.
         * @param message MissingParameter
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.MissingParameter, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this MissingParameter to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for MissingParameter
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an InvalidParameterValue. */
    interface IInvalidParameterValue {

        /** InvalidParameterValue parameter */
        parameter?: (string|null);

        /** InvalidParameterValue value */
        value?: (string|null);

        /** InvalidParameterValue explanation */
        explanation?: (string|null);
    }

    /** Represents an InvalidParameterValue. */
    class InvalidParameterValue implements IInvalidParameterValue {

        /**
         * Constructs a new InvalidParameterValue.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IInvalidParameterValue);

        /** InvalidParameterValue parameter. */
        public parameter: string;

        /** InvalidParameterValue value. */
        public value: string;

        /** InvalidParameterValue explanation. */
        public explanation?: (string|null);

        /**
         * Creates a new InvalidParameterValue instance using the specified properties.
         * @param [properties] Properties to set
         * @returns InvalidParameterValue instance
         */
        public static create(properties?: database_driver_v1.IInvalidParameterValue): database_driver_v1.InvalidParameterValue;

        /**
         * Encodes the specified InvalidParameterValue message. Does not implicitly {@link database_driver_v1.InvalidParameterValue.verify|verify} messages.
         * @param message InvalidParameterValue message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IInvalidParameterValue, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified InvalidParameterValue message, length delimited. Does not implicitly {@link database_driver_v1.InvalidParameterValue.verify|verify} messages.
         * @param message InvalidParameterValue message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IInvalidParameterValue, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an InvalidParameterValue message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns InvalidParameterValue
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.InvalidParameterValue;

        /**
         * Decodes an InvalidParameterValue message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns InvalidParameterValue
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.InvalidParameterValue;

        /**
         * Verifies an InvalidParameterValue message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an InvalidParameterValue message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns InvalidParameterValue
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.InvalidParameterValue;

        /**
         * Creates a plain object from an InvalidParameterValue message. Also converts values to other types if specified.
         * @param message InvalidParameterValue
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.InvalidParameterValue, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this InvalidParameterValue to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for InvalidParameterValue
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DriverError. */
    interface IDriverError {

        /** DriverError authError */
        authError?: (database_driver_v1.IAuthenticationError|null);

        /** DriverError genericError */
        genericError?: (database_driver_v1.IGenericError|null);

        /** DriverError internalError */
        internalError?: (database_driver_v1.IInternalError|null);

        /** DriverError missingParameter */
        missingParameter?: (database_driver_v1.IMissingParameter|null);

        /** DriverError invalidParameterValue */
        invalidParameterValue?: (database_driver_v1.IInvalidParameterValue|null);

        /** DriverError loginError */
        loginError?: (database_driver_v1.ILoginError|null);
    }

    /** Represents a DriverError. */
    class DriverError implements IDriverError {

        /**
         * Constructs a new DriverError.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDriverError);

        /** DriverError authError. */
        public authError?: (database_driver_v1.IAuthenticationError|null);

        /** DriverError genericError. */
        public genericError?: (database_driver_v1.IGenericError|null);

        /** DriverError internalError. */
        public internalError?: (database_driver_v1.IInternalError|null);

        /** DriverError missingParameter. */
        public missingParameter?: (database_driver_v1.IMissingParameter|null);

        /** DriverError invalidParameterValue. */
        public invalidParameterValue?: (database_driver_v1.IInvalidParameterValue|null);

        /** DriverError loginError. */
        public loginError?: (database_driver_v1.ILoginError|null);

        /** DriverError errorType. */
        public errorType?: ("authError"|"genericError"|"internalError"|"missingParameter"|"invalidParameterValue"|"loginError");

        /**
         * Creates a new DriverError instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DriverError instance
         */
        public static create(properties?: database_driver_v1.IDriverError): database_driver_v1.DriverError;

        /**
         * Encodes the specified DriverError message. Does not implicitly {@link database_driver_v1.DriverError.verify|verify} messages.
         * @param message DriverError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDriverError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DriverError message, length delimited. Does not implicitly {@link database_driver_v1.DriverError.verify|verify} messages.
         * @param message DriverError message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDriverError, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DriverError message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DriverError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DriverError;

        /**
         * Decodes a DriverError message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DriverError
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DriverError;

        /**
         * Verifies a DriverError message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DriverError message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DriverError
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DriverError;

        /**
         * Creates a plain object from a DriverError message. Also converts values to other types if specified.
         * @param message DriverError
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DriverError, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DriverError to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DriverError
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ErrorTraceEntry. */
    interface IErrorTraceEntry {

        /** ErrorTraceEntry file */
        file?: (string|null);

        /** ErrorTraceEntry line */
        line?: (number|null);

        /** ErrorTraceEntry column */
        column?: (number|null);

        /** ErrorTraceEntry message */
        message?: (string|null);
    }

    /** Represents an ErrorTraceEntry. */
    class ErrorTraceEntry implements IErrorTraceEntry {

        /**
         * Constructs a new ErrorTraceEntry.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IErrorTraceEntry);

        /** ErrorTraceEntry file. */
        public file: string;

        /** ErrorTraceEntry line. */
        public line: number;

        /** ErrorTraceEntry column. */
        public column: number;

        /** ErrorTraceEntry message. */
        public message: string;

        /**
         * Creates a new ErrorTraceEntry instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ErrorTraceEntry instance
         */
        public static create(properties?: database_driver_v1.IErrorTraceEntry): database_driver_v1.ErrorTraceEntry;

        /**
         * Encodes the specified ErrorTraceEntry message. Does not implicitly {@link database_driver_v1.ErrorTraceEntry.verify|verify} messages.
         * @param message ErrorTraceEntry message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IErrorTraceEntry, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ErrorTraceEntry message, length delimited. Does not implicitly {@link database_driver_v1.ErrorTraceEntry.verify|verify} messages.
         * @param message ErrorTraceEntry message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IErrorTraceEntry, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ErrorTraceEntry message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ErrorTraceEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ErrorTraceEntry;

        /**
         * Decodes an ErrorTraceEntry message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ErrorTraceEntry
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ErrorTraceEntry;

        /**
         * Verifies an ErrorTraceEntry message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ErrorTraceEntry message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ErrorTraceEntry
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ErrorTraceEntry;

        /**
         * Creates a plain object from an ErrorTraceEntry message. Also converts values to other types if specified.
         * @param message ErrorTraceEntry
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ErrorTraceEntry, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ErrorTraceEntry to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ErrorTraceEntry
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DriverException. */
    interface IDriverException {

        /** DriverException message */
        message?: (string|null);

        /** DriverException statusCode */
        statusCode?: (database_driver_v1.StatusCode|null);

        /** DriverException error */
        error?: (database_driver_v1.IDriverError|null);

        /** DriverException errorTrace */
        errorTrace?: (database_driver_v1.IErrorTraceEntry[]|null);

        /** DriverException vendorCode */
        vendorCode?: (number|null);

        /** DriverException sqlState */
        sqlState?: (string|null);

        /** DriverException rootCause */
        rootCause?: (string|null);
    }

    /** Represents a DriverException. */
    class DriverException implements IDriverException {

        /**
         * Constructs a new DriverException.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDriverException);

        /** DriverException message. */
        public message: string;

        /** DriverException statusCode. */
        public statusCode: database_driver_v1.StatusCode;

        /** DriverException error. */
        public error?: (database_driver_v1.IDriverError|null);

        /** DriverException errorTrace. */
        public errorTrace: database_driver_v1.IErrorTraceEntry[];

        /** DriverException vendorCode. */
        public vendorCode?: (number|null);

        /** DriverException sqlState. */
        public sqlState?: (string|null);

        /** DriverException rootCause. */
        public rootCause?: (string|null);

        /**
         * Creates a new DriverException instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DriverException instance
         */
        public static create(properties?: database_driver_v1.IDriverException): database_driver_v1.DriverException;

        /**
         * Encodes the specified DriverException message. Does not implicitly {@link database_driver_v1.DriverException.verify|verify} messages.
         * @param message DriverException message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDriverException, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DriverException message, length delimited. Does not implicitly {@link database_driver_v1.DriverException.verify|verify} messages.
         * @param message DriverException message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDriverException, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DriverException message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DriverException
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DriverException;

        /**
         * Decodes a DriverException message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DriverException
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DriverException;

        /**
         * Verifies a DriverException message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DriverException message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DriverException
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DriverException;

        /**
         * Creates a plain object from a DriverException message. Also converts values to other types if specified.
         * @param message DriverException
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DriverException, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DriverException to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DriverException
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ColumnMetadata. */
    interface IColumnMetadata {

        /** ColumnMetadata name */
        name?: (string|null);

        /** ColumnMetadata type */
        type?: (string|null);

        /** ColumnMetadata precision */
        precision?: (number|Long|null);

        /** ColumnMetadata scale */
        scale?: (number|Long|null);

        /** ColumnMetadata length */
        length?: (number|Long|null);

        /** ColumnMetadata byteLength */
        byteLength?: (number|Long|null);

        /** ColumnMetadata nullable */
        nullable?: (boolean|null);
    }

    /** Represents a ColumnMetadata. */
    class ColumnMetadata implements IColumnMetadata {

        /**
         * Constructs a new ColumnMetadata.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IColumnMetadata);

        /** ColumnMetadata name. */
        public name: string;

        /** ColumnMetadata type. */
        public type: string;

        /** ColumnMetadata precision. */
        public precision?: (number|Long|null);

        /** ColumnMetadata scale. */
        public scale?: (number|Long|null);

        /** ColumnMetadata length. */
        public length?: (number|Long|null);

        /** ColumnMetadata byteLength. */
        public byteLength?: (number|Long|null);

        /** ColumnMetadata nullable. */
        public nullable: boolean;

        /**
         * Creates a new ColumnMetadata instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ColumnMetadata instance
         */
        public static create(properties?: database_driver_v1.IColumnMetadata): database_driver_v1.ColumnMetadata;

        /**
         * Encodes the specified ColumnMetadata message. Does not implicitly {@link database_driver_v1.ColumnMetadata.verify|verify} messages.
         * @param message ColumnMetadata message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IColumnMetadata, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ColumnMetadata message, length delimited. Does not implicitly {@link database_driver_v1.ColumnMetadata.verify|verify} messages.
         * @param message ColumnMetadata message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IColumnMetadata, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ColumnMetadata message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ColumnMetadata
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ColumnMetadata;

        /**
         * Decodes a ColumnMetadata message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ColumnMetadata
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ColumnMetadata;

        /**
         * Verifies a ColumnMetadata message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ColumnMetadata message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ColumnMetadata
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ColumnMetadata;

        /**
         * Creates a plain object from a ColumnMetadata message. Also converts values to other types if specified.
         * @param message ColumnMetadata
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ColumnMetadata, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ColumnMetadata to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ColumnMetadata
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ExecuteResult. */
    interface IExecuteResult {

        /** ExecuteResult stream */
        stream?: (database_driver_v1.IArrowArrayStreamPtr|null);

        /** ExecuteResult rowsAffected */
        rowsAffected?: (number|Long|null);

        /** ExecuteResult queryId */
        queryId?: (string|null);

        /** ExecuteResult columns */
        columns?: (database_driver_v1.IColumnMetadata[]|null);

        /** ExecuteResult statementTypeId */
        statementTypeId?: (number|Long|null);

        /** ExecuteResult query */
        query?: (string|null);

        /** ExecuteResult sqlState */
        sqlState?: (string|null);
    }

    /** Represents an ExecuteResult. */
    class ExecuteResult implements IExecuteResult {

        /**
         * Constructs a new ExecuteResult.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IExecuteResult);

        /** ExecuteResult stream. */
        public stream?: (database_driver_v1.IArrowArrayStreamPtr|null);

        /** ExecuteResult rowsAffected. */
        public rowsAffected?: (number|Long|null);

        /** ExecuteResult queryId. */
        public queryId: string;

        /** ExecuteResult columns. */
        public columns: database_driver_v1.IColumnMetadata[];

        /** ExecuteResult statementTypeId. */
        public statementTypeId?: (number|Long|null);

        /** ExecuteResult query. */
        public query: string;

        /** ExecuteResult sqlState. */
        public sqlState?: (string|null);

        /**
         * Creates a new ExecuteResult instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ExecuteResult instance
         */
        public static create(properties?: database_driver_v1.IExecuteResult): database_driver_v1.ExecuteResult;

        /**
         * Encodes the specified ExecuteResult message. Does not implicitly {@link database_driver_v1.ExecuteResult.verify|verify} messages.
         * @param message ExecuteResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IExecuteResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ExecuteResult message, length delimited. Does not implicitly {@link database_driver_v1.ExecuteResult.verify|verify} messages.
         * @param message ExecuteResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IExecuteResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ExecuteResult message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ExecuteResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ExecuteResult;

        /**
         * Decodes an ExecuteResult message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ExecuteResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ExecuteResult;

        /**
         * Verifies an ExecuteResult message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ExecuteResult message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ExecuteResult
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ExecuteResult;

        /**
         * Creates a plain object from an ExecuteResult message. Also converts values to other types if specified.
         * @param message ExecuteResult
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ExecuteResult, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ExecuteResult to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ExecuteResult
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a PartitionedResult. */
    interface IPartitionedResult {

        /** PartitionedResult schema */
        schema?: (number|Long|null);

        /** PartitionedResult partitions */
        partitions?: (Uint8Array[]|null);

        /** PartitionedResult rowsAffected */
        rowsAffected?: (number|Long|null);
    }

    /** Represents a PartitionedResult. */
    class PartitionedResult implements IPartitionedResult {

        /**
         * Constructs a new PartitionedResult.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IPartitionedResult);

        /** PartitionedResult schema. */
        public schema: (number|Long);

        /** PartitionedResult partitions. */
        public partitions: Uint8Array[];

        /** PartitionedResult rowsAffected. */
        public rowsAffected?: (number|Long|null);

        /**
         * Creates a new PartitionedResult instance using the specified properties.
         * @param [properties] Properties to set
         * @returns PartitionedResult instance
         */
        public static create(properties?: database_driver_v1.IPartitionedResult): database_driver_v1.PartitionedResult;

        /**
         * Encodes the specified PartitionedResult message. Does not implicitly {@link database_driver_v1.PartitionedResult.verify|verify} messages.
         * @param message PartitionedResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IPartitionedResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified PartitionedResult message, length delimited. Does not implicitly {@link database_driver_v1.PartitionedResult.verify|verify} messages.
         * @param message PartitionedResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IPartitionedResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a PartitionedResult message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns PartitionedResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.PartitionedResult;

        /**
         * Decodes a PartitionedResult message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns PartitionedResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.PartitionedResult;

        /**
         * Verifies a PartitionedResult message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a PartitionedResult message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns PartitionedResult
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.PartitionedResult;

        /**
         * Creates a plain object from a PartitionedResult message. Also converts values to other types if specified.
         * @param message PartitionedResult
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.PartitionedResult, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this PartitionedResult to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for PartitionedResult
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseHandle. */
    interface IDatabaseHandle {

        /** DatabaseHandle id */
        id?: (number|Long|null);

        /** DatabaseHandle magic */
        magic?: (number|Long|null);
    }

    /** Represents a DatabaseHandle. */
    class DatabaseHandle implements IDatabaseHandle {

        /**
         * Constructs a new DatabaseHandle.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseHandle);

        /** DatabaseHandle id. */
        public id: (number|Long);

        /** DatabaseHandle magic. */
        public magic: (number|Long);

        /**
         * Creates a new DatabaseHandle instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseHandle instance
         */
        public static create(properties?: database_driver_v1.IDatabaseHandle): database_driver_v1.DatabaseHandle;

        /**
         * Encodes the specified DatabaseHandle message. Does not implicitly {@link database_driver_v1.DatabaseHandle.verify|verify} messages.
         * @param message DatabaseHandle message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseHandle, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseHandle message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseHandle.verify|verify} messages.
         * @param message DatabaseHandle message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseHandle, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseHandle message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseHandle
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseHandle;

        /**
         * Decodes a DatabaseHandle message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseHandle
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseHandle;

        /**
         * Verifies a DatabaseHandle message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseHandle message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseHandle
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseHandle;

        /**
         * Creates a plain object from a DatabaseHandle message. Also converts values to other types if specified.
         * @param message DatabaseHandle
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseHandle, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseHandle to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseHandle
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionHandle. */
    interface IConnectionHandle {

        /** ConnectionHandle id */
        id?: (number|Long|null);

        /** ConnectionHandle magic */
        magic?: (number|Long|null);
    }

    /** Represents a ConnectionHandle. */
    class ConnectionHandle implements IConnectionHandle {

        /**
         * Constructs a new ConnectionHandle.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionHandle);

        /** ConnectionHandle id. */
        public id: (number|Long);

        /** ConnectionHandle magic. */
        public magic: (number|Long);

        /**
         * Creates a new ConnectionHandle instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionHandle instance
         */
        public static create(properties?: database_driver_v1.IConnectionHandle): database_driver_v1.ConnectionHandle;

        /**
         * Encodes the specified ConnectionHandle message. Does not implicitly {@link database_driver_v1.ConnectionHandle.verify|verify} messages.
         * @param message ConnectionHandle message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionHandle, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionHandle message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionHandle.verify|verify} messages.
         * @param message ConnectionHandle message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionHandle, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionHandle message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionHandle
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionHandle;

        /**
         * Decodes a ConnectionHandle message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionHandle
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionHandle;

        /**
         * Verifies a ConnectionHandle message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionHandle message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionHandle
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionHandle;

        /**
         * Creates a plain object from a ConnectionHandle message. Also converts values to other types if specified.
         * @param message ConnectionHandle
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionHandle, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionHandle to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionHandle
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementHandle. */
    interface IStatementHandle {

        /** StatementHandle id */
        id?: (number|Long|null);

        /** StatementHandle magic */
        magic?: (number|Long|null);
    }

    /** Represents a StatementHandle. */
    class StatementHandle implements IStatementHandle {

        /**
         * Constructs a new StatementHandle.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementHandle);

        /** StatementHandle id. */
        public id: (number|Long);

        /** StatementHandle magic. */
        public magic: (number|Long);

        /**
         * Creates a new StatementHandle instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementHandle instance
         */
        public static create(properties?: database_driver_v1.IStatementHandle): database_driver_v1.StatementHandle;

        /**
         * Encodes the specified StatementHandle message. Does not implicitly {@link database_driver_v1.StatementHandle.verify|verify} messages.
         * @param message StatementHandle message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementHandle, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementHandle message, length delimited. Does not implicitly {@link database_driver_v1.StatementHandle.verify|verify} messages.
         * @param message StatementHandle message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementHandle, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementHandle message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementHandle
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementHandle;

        /**
         * Decodes a StatementHandle message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementHandle
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementHandle;

        /**
         * Verifies a StatementHandle message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementHandle message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementHandle
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementHandle;

        /**
         * Creates a plain object from a StatementHandle message. Also converts values to other types if specified.
         * @param message StatementHandle
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementHandle, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementHandle to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementHandle
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ArrowArrayStreamPtr. */
    interface IArrowArrayStreamPtr {

        /** ArrowArrayStreamPtr value */
        value?: (Uint8Array|null);
    }

    /** Represents an ArrowArrayStreamPtr. */
    class ArrowArrayStreamPtr implements IArrowArrayStreamPtr {

        /**
         * Constructs a new ArrowArrayStreamPtr.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IArrowArrayStreamPtr);

        /** ArrowArrayStreamPtr value. */
        public value: Uint8Array;

        /**
         * Creates a new ArrowArrayStreamPtr instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ArrowArrayStreamPtr instance
         */
        public static create(properties?: database_driver_v1.IArrowArrayStreamPtr): database_driver_v1.ArrowArrayStreamPtr;

        /**
         * Encodes the specified ArrowArrayStreamPtr message. Does not implicitly {@link database_driver_v1.ArrowArrayStreamPtr.verify|verify} messages.
         * @param message ArrowArrayStreamPtr message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IArrowArrayStreamPtr, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ArrowArrayStreamPtr message, length delimited. Does not implicitly {@link database_driver_v1.ArrowArrayStreamPtr.verify|verify} messages.
         * @param message ArrowArrayStreamPtr message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IArrowArrayStreamPtr, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ArrowArrayStreamPtr message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ArrowArrayStreamPtr
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ArrowArrayStreamPtr;

        /**
         * Decodes an ArrowArrayStreamPtr message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ArrowArrayStreamPtr
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ArrowArrayStreamPtr;

        /**
         * Verifies an ArrowArrayStreamPtr message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ArrowArrayStreamPtr message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ArrowArrayStreamPtr
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ArrowArrayStreamPtr;

        /**
         * Creates a plain object from an ArrowArrayStreamPtr message. Also converts values to other types if specified.
         * @param message ArrowArrayStreamPtr
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ArrowArrayStreamPtr, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ArrowArrayStreamPtr to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ArrowArrayStreamPtr
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an ArrowSchemaPtr. */
    interface IArrowSchemaPtr {

        /** ArrowSchemaPtr value */
        value?: (Uint8Array|null);
    }

    /** Represents an ArrowSchemaPtr. */
    class ArrowSchemaPtr implements IArrowSchemaPtr {

        /**
         * Constructs a new ArrowSchemaPtr.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IArrowSchemaPtr);

        /** ArrowSchemaPtr value. */
        public value: Uint8Array;

        /**
         * Creates a new ArrowSchemaPtr instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ArrowSchemaPtr instance
         */
        public static create(properties?: database_driver_v1.IArrowSchemaPtr): database_driver_v1.ArrowSchemaPtr;

        /**
         * Encodes the specified ArrowSchemaPtr message. Does not implicitly {@link database_driver_v1.ArrowSchemaPtr.verify|verify} messages.
         * @param message ArrowSchemaPtr message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IArrowSchemaPtr, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ArrowSchemaPtr message, length delimited. Does not implicitly {@link database_driver_v1.ArrowSchemaPtr.verify|verify} messages.
         * @param message ArrowSchemaPtr message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IArrowSchemaPtr, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an ArrowSchemaPtr message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ArrowSchemaPtr
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ArrowSchemaPtr;

        /**
         * Decodes an ArrowSchemaPtr message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ArrowSchemaPtr
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ArrowSchemaPtr;

        /**
         * Verifies an ArrowSchemaPtr message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an ArrowSchemaPtr message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ArrowSchemaPtr
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ArrowSchemaPtr;

        /**
         * Creates a plain object from an ArrowSchemaPtr message. Also converts values to other types if specified.
         * @param message ArrowSchemaPtr
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ArrowSchemaPtr, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ArrowSchemaPtr to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ArrowSchemaPtr
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a BinaryDataPtr. */
    interface IBinaryDataPtr {

        /** BinaryDataPtr value */
        value?: (Uint8Array|null);

        /** BinaryDataPtr length */
        length?: (number|Long|null);
    }

    /** Represents a BinaryDataPtr. */
    class BinaryDataPtr implements IBinaryDataPtr {

        /**
         * Constructs a new BinaryDataPtr.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IBinaryDataPtr);

        /** BinaryDataPtr value. */
        public value: Uint8Array;

        /** BinaryDataPtr length. */
        public length: (number|Long);

        /**
         * Creates a new BinaryDataPtr instance using the specified properties.
         * @param [properties] Properties to set
         * @returns BinaryDataPtr instance
         */
        public static create(properties?: database_driver_v1.IBinaryDataPtr): database_driver_v1.BinaryDataPtr;

        /**
         * Encodes the specified BinaryDataPtr message. Does not implicitly {@link database_driver_v1.BinaryDataPtr.verify|verify} messages.
         * @param message BinaryDataPtr message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IBinaryDataPtr, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified BinaryDataPtr message, length delimited. Does not implicitly {@link database_driver_v1.BinaryDataPtr.verify|verify} messages.
         * @param message BinaryDataPtr message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IBinaryDataPtr, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a BinaryDataPtr message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns BinaryDataPtr
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.BinaryDataPtr;

        /**
         * Decodes a BinaryDataPtr message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns BinaryDataPtr
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.BinaryDataPtr;

        /**
         * Verifies a BinaryDataPtr message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a BinaryDataPtr message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns BinaryDataPtr
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.BinaryDataPtr;

        /**
         * Creates a plain object from a BinaryDataPtr message. Also converts values to other types if specified.
         * @param message BinaryDataPtr
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.BinaryDataPtr, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this BinaryDataPtr to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for BinaryDataPtr
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a QueryBindings. */
    interface IQueryBindings {

        /** QueryBindings json */
        json?: (database_driver_v1.IBinaryDataPtr|null);

        /** QueryBindings csv */
        csv?: (database_driver_v1.IBinaryDataPtr|null);
    }

    /** Represents a QueryBindings. */
    class QueryBindings implements IQueryBindings {

        /**
         * Constructs a new QueryBindings.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IQueryBindings);

        /** QueryBindings json. */
        public json?: (database_driver_v1.IBinaryDataPtr|null);

        /** QueryBindings csv. */
        public csv?: (database_driver_v1.IBinaryDataPtr|null);

        /** QueryBindings bindingType. */
        public bindingType?: ("json"|"csv");

        /**
         * Creates a new QueryBindings instance using the specified properties.
         * @param [properties] Properties to set
         * @returns QueryBindings instance
         */
        public static create(properties?: database_driver_v1.IQueryBindings): database_driver_v1.QueryBindings;

        /**
         * Encodes the specified QueryBindings message. Does not implicitly {@link database_driver_v1.QueryBindings.verify|verify} messages.
         * @param message QueryBindings message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IQueryBindings, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified QueryBindings message, length delimited. Does not implicitly {@link database_driver_v1.QueryBindings.verify|verify} messages.
         * @param message QueryBindings message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IQueryBindings, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a QueryBindings message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns QueryBindings
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.QueryBindings;

        /**
         * Decodes a QueryBindings message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns QueryBindings
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.QueryBindings;

        /**
         * Verifies a QueryBindings message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a QueryBindings message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns QueryBindings
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.QueryBindings;

        /**
         * Creates a plain object from a QueryBindings message. Also converts values to other types if specified.
         * @param message QueryBindings
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.QueryBindings, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this QueryBindings to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for QueryBindings
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseNewRequest. */
    interface IDatabaseNewRequest {
    }

    /** Represents a DatabaseNewRequest. */
    class DatabaseNewRequest implements IDatabaseNewRequest {

        /**
         * Constructs a new DatabaseNewRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseNewRequest);

        /**
         * Creates a new DatabaseNewRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseNewRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseNewRequest): database_driver_v1.DatabaseNewRequest;

        /**
         * Encodes the specified DatabaseNewRequest message. Does not implicitly {@link database_driver_v1.DatabaseNewRequest.verify|verify} messages.
         * @param message DatabaseNewRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseNewRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseNewRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseNewRequest.verify|verify} messages.
         * @param message DatabaseNewRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseNewRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseNewRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseNewRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseNewRequest;

        /**
         * Decodes a DatabaseNewRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseNewRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseNewRequest;

        /**
         * Verifies a DatabaseNewRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseNewRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseNewRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseNewRequest;

        /**
         * Creates a plain object from a DatabaseNewRequest message. Also converts values to other types if specified.
         * @param message DatabaseNewRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseNewRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseNewRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseNewRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseNewResponse. */
    interface IDatabaseNewResponse {

        /** DatabaseNewResponse dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);
    }

    /** Represents a DatabaseNewResponse. */
    class DatabaseNewResponse implements IDatabaseNewResponse {

        /**
         * Constructs a new DatabaseNewResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseNewResponse);

        /** DatabaseNewResponse dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /**
         * Creates a new DatabaseNewResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseNewResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseNewResponse): database_driver_v1.DatabaseNewResponse;

        /**
         * Encodes the specified DatabaseNewResponse message. Does not implicitly {@link database_driver_v1.DatabaseNewResponse.verify|verify} messages.
         * @param message DatabaseNewResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseNewResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseNewResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseNewResponse.verify|verify} messages.
         * @param message DatabaseNewResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseNewResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseNewResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseNewResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseNewResponse;

        /**
         * Decodes a DatabaseNewResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseNewResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseNewResponse;

        /**
         * Verifies a DatabaseNewResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseNewResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseNewResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseNewResponse;

        /**
         * Creates a plain object from a DatabaseNewResponse message. Also converts values to other types if specified.
         * @param message DatabaseNewResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseNewResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseNewResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseNewResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionStringRequest. */
    interface IDatabaseSetOptionStringRequest {

        /** DatabaseSetOptionStringRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionStringRequest key */
        key?: (string|null);

        /** DatabaseSetOptionStringRequest value */
        value?: (string|null);
    }

    /** Represents a DatabaseSetOptionStringRequest. */
    class DatabaseSetOptionStringRequest implements IDatabaseSetOptionStringRequest {

        /**
         * Constructs a new DatabaseSetOptionStringRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionStringRequest);

        /** DatabaseSetOptionStringRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionStringRequest key. */
        public key: string;

        /** DatabaseSetOptionStringRequest value. */
        public value: string;

        /**
         * Creates a new DatabaseSetOptionStringRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionStringRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionStringRequest): database_driver_v1.DatabaseSetOptionStringRequest;

        /**
         * Encodes the specified DatabaseSetOptionStringRequest message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionStringRequest.verify|verify} messages.
         * @param message DatabaseSetOptionStringRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionStringRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionStringRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionStringRequest.verify|verify} messages.
         * @param message DatabaseSetOptionStringRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionStringRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionStringRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionStringRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionStringRequest;

        /**
         * Decodes a DatabaseSetOptionStringRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionStringRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionStringRequest;

        /**
         * Verifies a DatabaseSetOptionStringRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionStringRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionStringRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionStringRequest;

        /**
         * Creates a plain object from a DatabaseSetOptionStringRequest message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionStringRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionStringRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionStringRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionStringRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionStringResponse. */
    interface IDatabaseSetOptionStringResponse {
    }

    /** Represents a DatabaseSetOptionStringResponse. */
    class DatabaseSetOptionStringResponse implements IDatabaseSetOptionStringResponse {

        /**
         * Constructs a new DatabaseSetOptionStringResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionStringResponse);

        /**
         * Creates a new DatabaseSetOptionStringResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionStringResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionStringResponse): database_driver_v1.DatabaseSetOptionStringResponse;

        /**
         * Encodes the specified DatabaseSetOptionStringResponse message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionStringResponse.verify|verify} messages.
         * @param message DatabaseSetOptionStringResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionStringResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionStringResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionStringResponse.verify|verify} messages.
         * @param message DatabaseSetOptionStringResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionStringResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionStringResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionStringResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionStringResponse;

        /**
         * Decodes a DatabaseSetOptionStringResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionStringResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionStringResponse;

        /**
         * Verifies a DatabaseSetOptionStringResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionStringResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionStringResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionStringResponse;

        /**
         * Creates a plain object from a DatabaseSetOptionStringResponse message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionStringResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionStringResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionStringResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionStringResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionBytesRequest. */
    interface IDatabaseSetOptionBytesRequest {

        /** DatabaseSetOptionBytesRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionBytesRequest key */
        key?: (string|null);

        /** DatabaseSetOptionBytesRequest value */
        value?: (Uint8Array|null);
    }

    /** Represents a DatabaseSetOptionBytesRequest. */
    class DatabaseSetOptionBytesRequest implements IDatabaseSetOptionBytesRequest {

        /**
         * Constructs a new DatabaseSetOptionBytesRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionBytesRequest);

        /** DatabaseSetOptionBytesRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionBytesRequest key. */
        public key: string;

        /** DatabaseSetOptionBytesRequest value. */
        public value: Uint8Array;

        /**
         * Creates a new DatabaseSetOptionBytesRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionBytesRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionBytesRequest): database_driver_v1.DatabaseSetOptionBytesRequest;

        /**
         * Encodes the specified DatabaseSetOptionBytesRequest message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionBytesRequest.verify|verify} messages.
         * @param message DatabaseSetOptionBytesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionBytesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionBytesRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionBytesRequest.verify|verify} messages.
         * @param message DatabaseSetOptionBytesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionBytesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionBytesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionBytesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionBytesRequest;

        /**
         * Decodes a DatabaseSetOptionBytesRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionBytesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionBytesRequest;

        /**
         * Verifies a DatabaseSetOptionBytesRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionBytesRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionBytesRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionBytesRequest;

        /**
         * Creates a plain object from a DatabaseSetOptionBytesRequest message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionBytesRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionBytesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionBytesRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionBytesRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionBytesResponse. */
    interface IDatabaseSetOptionBytesResponse {
    }

    /** Represents a DatabaseSetOptionBytesResponse. */
    class DatabaseSetOptionBytesResponse implements IDatabaseSetOptionBytesResponse {

        /**
         * Constructs a new DatabaseSetOptionBytesResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionBytesResponse);

        /**
         * Creates a new DatabaseSetOptionBytesResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionBytesResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionBytesResponse): database_driver_v1.DatabaseSetOptionBytesResponse;

        /**
         * Encodes the specified DatabaseSetOptionBytesResponse message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionBytesResponse.verify|verify} messages.
         * @param message DatabaseSetOptionBytesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionBytesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionBytesResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionBytesResponse.verify|verify} messages.
         * @param message DatabaseSetOptionBytesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionBytesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionBytesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionBytesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionBytesResponse;

        /**
         * Decodes a DatabaseSetOptionBytesResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionBytesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionBytesResponse;

        /**
         * Verifies a DatabaseSetOptionBytesResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionBytesResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionBytesResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionBytesResponse;

        /**
         * Creates a plain object from a DatabaseSetOptionBytesResponse message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionBytesResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionBytesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionBytesResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionBytesResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionIntRequest. */
    interface IDatabaseSetOptionIntRequest {

        /** DatabaseSetOptionIntRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionIntRequest key */
        key?: (string|null);

        /** DatabaseSetOptionIntRequest value */
        value?: (number|Long|null);
    }

    /** Represents a DatabaseSetOptionIntRequest. */
    class DatabaseSetOptionIntRequest implements IDatabaseSetOptionIntRequest {

        /**
         * Constructs a new DatabaseSetOptionIntRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionIntRequest);

        /** DatabaseSetOptionIntRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionIntRequest key. */
        public key: string;

        /** DatabaseSetOptionIntRequest value. */
        public value: (number|Long);

        /**
         * Creates a new DatabaseSetOptionIntRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionIntRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionIntRequest): database_driver_v1.DatabaseSetOptionIntRequest;

        /**
         * Encodes the specified DatabaseSetOptionIntRequest message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionIntRequest.verify|verify} messages.
         * @param message DatabaseSetOptionIntRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionIntRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionIntRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionIntRequest.verify|verify} messages.
         * @param message DatabaseSetOptionIntRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionIntRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionIntRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionIntRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionIntRequest;

        /**
         * Decodes a DatabaseSetOptionIntRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionIntRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionIntRequest;

        /**
         * Verifies a DatabaseSetOptionIntRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionIntRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionIntRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionIntRequest;

        /**
         * Creates a plain object from a DatabaseSetOptionIntRequest message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionIntRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionIntRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionIntRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionIntRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionIntResponse. */
    interface IDatabaseSetOptionIntResponse {
    }

    /** Represents a DatabaseSetOptionIntResponse. */
    class DatabaseSetOptionIntResponse implements IDatabaseSetOptionIntResponse {

        /**
         * Constructs a new DatabaseSetOptionIntResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionIntResponse);

        /**
         * Creates a new DatabaseSetOptionIntResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionIntResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionIntResponse): database_driver_v1.DatabaseSetOptionIntResponse;

        /**
         * Encodes the specified DatabaseSetOptionIntResponse message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionIntResponse.verify|verify} messages.
         * @param message DatabaseSetOptionIntResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionIntResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionIntResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionIntResponse.verify|verify} messages.
         * @param message DatabaseSetOptionIntResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionIntResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionIntResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionIntResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionIntResponse;

        /**
         * Decodes a DatabaseSetOptionIntResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionIntResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionIntResponse;

        /**
         * Verifies a DatabaseSetOptionIntResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionIntResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionIntResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionIntResponse;

        /**
         * Creates a plain object from a DatabaseSetOptionIntResponse message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionIntResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionIntResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionIntResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionIntResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionDoubleRequest. */
    interface IDatabaseSetOptionDoubleRequest {

        /** DatabaseSetOptionDoubleRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionDoubleRequest key */
        key?: (string|null);

        /** DatabaseSetOptionDoubleRequest value */
        value?: (number|null);
    }

    /** Represents a DatabaseSetOptionDoubleRequest. */
    class DatabaseSetOptionDoubleRequest implements IDatabaseSetOptionDoubleRequest {

        /**
         * Constructs a new DatabaseSetOptionDoubleRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionDoubleRequest);

        /** DatabaseSetOptionDoubleRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /** DatabaseSetOptionDoubleRequest key. */
        public key: string;

        /** DatabaseSetOptionDoubleRequest value. */
        public value: number;

        /**
         * Creates a new DatabaseSetOptionDoubleRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionDoubleRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionDoubleRequest): database_driver_v1.DatabaseSetOptionDoubleRequest;

        /**
         * Encodes the specified DatabaseSetOptionDoubleRequest message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionDoubleRequest.verify|verify} messages.
         * @param message DatabaseSetOptionDoubleRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionDoubleRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionDoubleRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionDoubleRequest.verify|verify} messages.
         * @param message DatabaseSetOptionDoubleRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionDoubleRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionDoubleRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionDoubleRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionDoubleRequest;

        /**
         * Decodes a DatabaseSetOptionDoubleRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionDoubleRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionDoubleRequest;

        /**
         * Verifies a DatabaseSetOptionDoubleRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionDoubleRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionDoubleRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionDoubleRequest;

        /**
         * Creates a plain object from a DatabaseSetOptionDoubleRequest message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionDoubleRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionDoubleRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionDoubleRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionDoubleRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseSetOptionDoubleResponse. */
    interface IDatabaseSetOptionDoubleResponse {
    }

    /** Represents a DatabaseSetOptionDoubleResponse. */
    class DatabaseSetOptionDoubleResponse implements IDatabaseSetOptionDoubleResponse {

        /**
         * Constructs a new DatabaseSetOptionDoubleResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseSetOptionDoubleResponse);

        /**
         * Creates a new DatabaseSetOptionDoubleResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseSetOptionDoubleResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseSetOptionDoubleResponse): database_driver_v1.DatabaseSetOptionDoubleResponse;

        /**
         * Encodes the specified DatabaseSetOptionDoubleResponse message. Does not implicitly {@link database_driver_v1.DatabaseSetOptionDoubleResponse.verify|verify} messages.
         * @param message DatabaseSetOptionDoubleResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseSetOptionDoubleResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseSetOptionDoubleResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseSetOptionDoubleResponse.verify|verify} messages.
         * @param message DatabaseSetOptionDoubleResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseSetOptionDoubleResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseSetOptionDoubleResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseSetOptionDoubleResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseSetOptionDoubleResponse;

        /**
         * Decodes a DatabaseSetOptionDoubleResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseSetOptionDoubleResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseSetOptionDoubleResponse;

        /**
         * Verifies a DatabaseSetOptionDoubleResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseSetOptionDoubleResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseSetOptionDoubleResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseSetOptionDoubleResponse;

        /**
         * Creates a plain object from a DatabaseSetOptionDoubleResponse message. Also converts values to other types if specified.
         * @param message DatabaseSetOptionDoubleResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseSetOptionDoubleResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseSetOptionDoubleResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseSetOptionDoubleResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseInitRequest. */
    interface IDatabaseInitRequest {

        /** DatabaseInitRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);
    }

    /** Represents a DatabaseInitRequest. */
    class DatabaseInitRequest implements IDatabaseInitRequest {

        /**
         * Constructs a new DatabaseInitRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseInitRequest);

        /** DatabaseInitRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /**
         * Creates a new DatabaseInitRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseInitRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseInitRequest): database_driver_v1.DatabaseInitRequest;

        /**
         * Encodes the specified DatabaseInitRequest message. Does not implicitly {@link database_driver_v1.DatabaseInitRequest.verify|verify} messages.
         * @param message DatabaseInitRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseInitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseInitRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseInitRequest.verify|verify} messages.
         * @param message DatabaseInitRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseInitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseInitRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseInitRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseInitRequest;

        /**
         * Decodes a DatabaseInitRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseInitRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseInitRequest;

        /**
         * Verifies a DatabaseInitRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseInitRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseInitRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseInitRequest;

        /**
         * Creates a plain object from a DatabaseInitRequest message. Also converts values to other types if specified.
         * @param message DatabaseInitRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseInitRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseInitRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseInitRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseInitResponse. */
    interface IDatabaseInitResponse {
    }

    /** Represents a DatabaseInitResponse. */
    class DatabaseInitResponse implements IDatabaseInitResponse {

        /**
         * Constructs a new DatabaseInitResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseInitResponse);

        /**
         * Creates a new DatabaseInitResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseInitResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseInitResponse): database_driver_v1.DatabaseInitResponse;

        /**
         * Encodes the specified DatabaseInitResponse message. Does not implicitly {@link database_driver_v1.DatabaseInitResponse.verify|verify} messages.
         * @param message DatabaseInitResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseInitResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseInitResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseInitResponse.verify|verify} messages.
         * @param message DatabaseInitResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseInitResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseInitResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseInitResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseInitResponse;

        /**
         * Decodes a DatabaseInitResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseInitResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseInitResponse;

        /**
         * Verifies a DatabaseInitResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseInitResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseInitResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseInitResponse;

        /**
         * Creates a plain object from a DatabaseInitResponse message. Also converts values to other types if specified.
         * @param message DatabaseInitResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseInitResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseInitResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseInitResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseReleaseRequest. */
    interface IDatabaseReleaseRequest {

        /** DatabaseReleaseRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);
    }

    /** Represents a DatabaseReleaseRequest. */
    class DatabaseReleaseRequest implements IDatabaseReleaseRequest {

        /**
         * Constructs a new DatabaseReleaseRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseReleaseRequest);

        /** DatabaseReleaseRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /**
         * Creates a new DatabaseReleaseRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseReleaseRequest instance
         */
        public static create(properties?: database_driver_v1.IDatabaseReleaseRequest): database_driver_v1.DatabaseReleaseRequest;

        /**
         * Encodes the specified DatabaseReleaseRequest message. Does not implicitly {@link database_driver_v1.DatabaseReleaseRequest.verify|verify} messages.
         * @param message DatabaseReleaseRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseReleaseRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseReleaseRequest message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseReleaseRequest.verify|verify} messages.
         * @param message DatabaseReleaseRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseReleaseRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseReleaseRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseReleaseRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseReleaseRequest;

        /**
         * Decodes a DatabaseReleaseRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseReleaseRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseReleaseRequest;

        /**
         * Verifies a DatabaseReleaseRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseReleaseRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseReleaseRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseReleaseRequest;

        /**
         * Creates a plain object from a DatabaseReleaseRequest message. Also converts values to other types if specified.
         * @param message DatabaseReleaseRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseReleaseRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseReleaseRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseReleaseRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DatabaseReleaseResponse. */
    interface IDatabaseReleaseResponse {
    }

    /** Represents a DatabaseReleaseResponse. */
    class DatabaseReleaseResponse implements IDatabaseReleaseResponse {

        /**
         * Constructs a new DatabaseReleaseResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IDatabaseReleaseResponse);

        /**
         * Creates a new DatabaseReleaseResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DatabaseReleaseResponse instance
         */
        public static create(properties?: database_driver_v1.IDatabaseReleaseResponse): database_driver_v1.DatabaseReleaseResponse;

        /**
         * Encodes the specified DatabaseReleaseResponse message. Does not implicitly {@link database_driver_v1.DatabaseReleaseResponse.verify|verify} messages.
         * @param message DatabaseReleaseResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IDatabaseReleaseResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DatabaseReleaseResponse message, length delimited. Does not implicitly {@link database_driver_v1.DatabaseReleaseResponse.verify|verify} messages.
         * @param message DatabaseReleaseResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IDatabaseReleaseResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DatabaseReleaseResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DatabaseReleaseResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.DatabaseReleaseResponse;

        /**
         * Decodes a DatabaseReleaseResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DatabaseReleaseResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.DatabaseReleaseResponse;

        /**
         * Verifies a DatabaseReleaseResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DatabaseReleaseResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DatabaseReleaseResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.DatabaseReleaseResponse;

        /**
         * Creates a plain object from a DatabaseReleaseResponse message. Also converts values to other types if specified.
         * @param message DatabaseReleaseResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.DatabaseReleaseResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DatabaseReleaseResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DatabaseReleaseResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionNewRequest. */
    interface IConnectionNewRequest {
    }

    /** Represents a ConnectionNewRequest. */
    class ConnectionNewRequest implements IConnectionNewRequest {

        /**
         * Constructs a new ConnectionNewRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionNewRequest);

        /**
         * Creates a new ConnectionNewRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionNewRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionNewRequest): database_driver_v1.ConnectionNewRequest;

        /**
         * Encodes the specified ConnectionNewRequest message. Does not implicitly {@link database_driver_v1.ConnectionNewRequest.verify|verify} messages.
         * @param message ConnectionNewRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionNewRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionNewRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionNewRequest.verify|verify} messages.
         * @param message ConnectionNewRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionNewRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionNewRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionNewRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionNewRequest;

        /**
         * Decodes a ConnectionNewRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionNewRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionNewRequest;

        /**
         * Verifies a ConnectionNewRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionNewRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionNewRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionNewRequest;

        /**
         * Creates a plain object from a ConnectionNewRequest message. Also converts values to other types if specified.
         * @param message ConnectionNewRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionNewRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionNewRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionNewRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionNewResponse. */
    interface IConnectionNewResponse {

        /** ConnectionNewResponse connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);
    }

    /** Represents a ConnectionNewResponse. */
    class ConnectionNewResponse implements IConnectionNewResponse {

        /**
         * Constructs a new ConnectionNewResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionNewResponse);

        /** ConnectionNewResponse connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /**
         * Creates a new ConnectionNewResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionNewResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionNewResponse): database_driver_v1.ConnectionNewResponse;

        /**
         * Encodes the specified ConnectionNewResponse message. Does not implicitly {@link database_driver_v1.ConnectionNewResponse.verify|verify} messages.
         * @param message ConnectionNewResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionNewResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionNewResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionNewResponse.verify|verify} messages.
         * @param message ConnectionNewResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionNewResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionNewResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionNewResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionNewResponse;

        /**
         * Decodes a ConnectionNewResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionNewResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionNewResponse;

        /**
         * Verifies a ConnectionNewResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionNewResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionNewResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionNewResponse;

        /**
         * Creates a plain object from a ConnectionNewResponse message. Also converts values to other types if specified.
         * @param message ConnectionNewResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionNewResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionNewResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionNewResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionStringRequest. */
    interface IConnectionSetOptionStringRequest {

        /** ConnectionSetOptionStringRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionStringRequest key */
        key?: (string|null);

        /** ConnectionSetOptionStringRequest value */
        value?: (string|null);
    }

    /** Represents a ConnectionSetOptionStringRequest. */
    class ConnectionSetOptionStringRequest implements IConnectionSetOptionStringRequest {

        /**
         * Constructs a new ConnectionSetOptionStringRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionStringRequest);

        /** ConnectionSetOptionStringRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionStringRequest key. */
        public key: string;

        /** ConnectionSetOptionStringRequest value. */
        public value: string;

        /**
         * Creates a new ConnectionSetOptionStringRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionStringRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionStringRequest): database_driver_v1.ConnectionSetOptionStringRequest;

        /**
         * Encodes the specified ConnectionSetOptionStringRequest message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionStringRequest.verify|verify} messages.
         * @param message ConnectionSetOptionStringRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionStringRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionStringRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionStringRequest.verify|verify} messages.
         * @param message ConnectionSetOptionStringRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionStringRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionStringRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionStringRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionStringRequest;

        /**
         * Decodes a ConnectionSetOptionStringRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionStringRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionStringRequest;

        /**
         * Verifies a ConnectionSetOptionStringRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionStringRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionStringRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionStringRequest;

        /**
         * Creates a plain object from a ConnectionSetOptionStringRequest message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionStringRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionStringRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionStringRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionStringRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionStringResponse. */
    interface IConnectionSetOptionStringResponse {
    }

    /** Represents a ConnectionSetOptionStringResponse. */
    class ConnectionSetOptionStringResponse implements IConnectionSetOptionStringResponse {

        /**
         * Constructs a new ConnectionSetOptionStringResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionStringResponse);

        /**
         * Creates a new ConnectionSetOptionStringResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionStringResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionStringResponse): database_driver_v1.ConnectionSetOptionStringResponse;

        /**
         * Encodes the specified ConnectionSetOptionStringResponse message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionStringResponse.verify|verify} messages.
         * @param message ConnectionSetOptionStringResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionStringResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionStringResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionStringResponse.verify|verify} messages.
         * @param message ConnectionSetOptionStringResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionStringResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionStringResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionStringResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionStringResponse;

        /**
         * Decodes a ConnectionSetOptionStringResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionStringResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionStringResponse;

        /**
         * Verifies a ConnectionSetOptionStringResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionStringResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionStringResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionStringResponse;

        /**
         * Creates a plain object from a ConnectionSetOptionStringResponse message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionStringResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionStringResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionStringResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionStringResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionBytesRequest. */
    interface IConnectionSetOptionBytesRequest {

        /** ConnectionSetOptionBytesRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionBytesRequest key */
        key?: (string|null);

        /** ConnectionSetOptionBytesRequest value */
        value?: (Uint8Array|null);
    }

    /** Represents a ConnectionSetOptionBytesRequest. */
    class ConnectionSetOptionBytesRequest implements IConnectionSetOptionBytesRequest {

        /**
         * Constructs a new ConnectionSetOptionBytesRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionBytesRequest);

        /** ConnectionSetOptionBytesRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionBytesRequest key. */
        public key: string;

        /** ConnectionSetOptionBytesRequest value. */
        public value: Uint8Array;

        /**
         * Creates a new ConnectionSetOptionBytesRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionBytesRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionBytesRequest): database_driver_v1.ConnectionSetOptionBytesRequest;

        /**
         * Encodes the specified ConnectionSetOptionBytesRequest message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionBytesRequest.verify|verify} messages.
         * @param message ConnectionSetOptionBytesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionBytesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionBytesRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionBytesRequest.verify|verify} messages.
         * @param message ConnectionSetOptionBytesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionBytesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionBytesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionBytesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionBytesRequest;

        /**
         * Decodes a ConnectionSetOptionBytesRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionBytesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionBytesRequest;

        /**
         * Verifies a ConnectionSetOptionBytesRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionBytesRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionBytesRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionBytesRequest;

        /**
         * Creates a plain object from a ConnectionSetOptionBytesRequest message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionBytesRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionBytesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionBytesRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionBytesRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionBytesResponse. */
    interface IConnectionSetOptionBytesResponse {
    }

    /** Represents a ConnectionSetOptionBytesResponse. */
    class ConnectionSetOptionBytesResponse implements IConnectionSetOptionBytesResponse {

        /**
         * Constructs a new ConnectionSetOptionBytesResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionBytesResponse);

        /**
         * Creates a new ConnectionSetOptionBytesResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionBytesResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionBytesResponse): database_driver_v1.ConnectionSetOptionBytesResponse;

        /**
         * Encodes the specified ConnectionSetOptionBytesResponse message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionBytesResponse.verify|verify} messages.
         * @param message ConnectionSetOptionBytesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionBytesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionBytesResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionBytesResponse.verify|verify} messages.
         * @param message ConnectionSetOptionBytesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionBytesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionBytesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionBytesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionBytesResponse;

        /**
         * Decodes a ConnectionSetOptionBytesResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionBytesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionBytesResponse;

        /**
         * Verifies a ConnectionSetOptionBytesResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionBytesResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionBytesResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionBytesResponse;

        /**
         * Creates a plain object from a ConnectionSetOptionBytesResponse message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionBytesResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionBytesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionBytesResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionBytesResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionIntRequest. */
    interface IConnectionSetOptionIntRequest {

        /** ConnectionSetOptionIntRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionIntRequest key */
        key?: (string|null);

        /** ConnectionSetOptionIntRequest value */
        value?: (number|Long|null);
    }

    /** Represents a ConnectionSetOptionIntRequest. */
    class ConnectionSetOptionIntRequest implements IConnectionSetOptionIntRequest {

        /**
         * Constructs a new ConnectionSetOptionIntRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionIntRequest);

        /** ConnectionSetOptionIntRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionIntRequest key. */
        public key: string;

        /** ConnectionSetOptionIntRequest value. */
        public value: (number|Long);

        /**
         * Creates a new ConnectionSetOptionIntRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionIntRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionIntRequest): database_driver_v1.ConnectionSetOptionIntRequest;

        /**
         * Encodes the specified ConnectionSetOptionIntRequest message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionIntRequest.verify|verify} messages.
         * @param message ConnectionSetOptionIntRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionIntRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionIntRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionIntRequest.verify|verify} messages.
         * @param message ConnectionSetOptionIntRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionIntRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionIntRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionIntRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionIntRequest;

        /**
         * Decodes a ConnectionSetOptionIntRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionIntRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionIntRequest;

        /**
         * Verifies a ConnectionSetOptionIntRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionIntRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionIntRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionIntRequest;

        /**
         * Creates a plain object from a ConnectionSetOptionIntRequest message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionIntRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionIntRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionIntRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionIntRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionIntResponse. */
    interface IConnectionSetOptionIntResponse {
    }

    /** Represents a ConnectionSetOptionIntResponse. */
    class ConnectionSetOptionIntResponse implements IConnectionSetOptionIntResponse {

        /**
         * Constructs a new ConnectionSetOptionIntResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionIntResponse);

        /**
         * Creates a new ConnectionSetOptionIntResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionIntResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionIntResponse): database_driver_v1.ConnectionSetOptionIntResponse;

        /**
         * Encodes the specified ConnectionSetOptionIntResponse message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionIntResponse.verify|verify} messages.
         * @param message ConnectionSetOptionIntResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionIntResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionIntResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionIntResponse.verify|verify} messages.
         * @param message ConnectionSetOptionIntResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionIntResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionIntResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionIntResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionIntResponse;

        /**
         * Decodes a ConnectionSetOptionIntResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionIntResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionIntResponse;

        /**
         * Verifies a ConnectionSetOptionIntResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionIntResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionIntResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionIntResponse;

        /**
         * Creates a plain object from a ConnectionSetOptionIntResponse message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionIntResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionIntResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionIntResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionIntResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionDoubleRequest. */
    interface IConnectionSetOptionDoubleRequest {

        /** ConnectionSetOptionDoubleRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionDoubleRequest key */
        key?: (string|null);

        /** ConnectionSetOptionDoubleRequest value */
        value?: (number|null);
    }

    /** Represents a ConnectionSetOptionDoubleRequest. */
    class ConnectionSetOptionDoubleRequest implements IConnectionSetOptionDoubleRequest {

        /**
         * Constructs a new ConnectionSetOptionDoubleRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionDoubleRequest);

        /** ConnectionSetOptionDoubleRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetOptionDoubleRequest key. */
        public key: string;

        /** ConnectionSetOptionDoubleRequest value. */
        public value: number;

        /**
         * Creates a new ConnectionSetOptionDoubleRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionDoubleRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionDoubleRequest): database_driver_v1.ConnectionSetOptionDoubleRequest;

        /**
         * Encodes the specified ConnectionSetOptionDoubleRequest message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionDoubleRequest.verify|verify} messages.
         * @param message ConnectionSetOptionDoubleRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionDoubleRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionDoubleRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionDoubleRequest.verify|verify} messages.
         * @param message ConnectionSetOptionDoubleRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionDoubleRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionDoubleRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionDoubleRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionDoubleRequest;

        /**
         * Decodes a ConnectionSetOptionDoubleRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionDoubleRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionDoubleRequest;

        /**
         * Verifies a ConnectionSetOptionDoubleRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionDoubleRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionDoubleRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionDoubleRequest;

        /**
         * Creates a plain object from a ConnectionSetOptionDoubleRequest message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionDoubleRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionDoubleRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionDoubleRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionDoubleRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetOptionDoubleResponse. */
    interface IConnectionSetOptionDoubleResponse {
    }

    /** Represents a ConnectionSetOptionDoubleResponse. */
    class ConnectionSetOptionDoubleResponse implements IConnectionSetOptionDoubleResponse {

        /**
         * Constructs a new ConnectionSetOptionDoubleResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetOptionDoubleResponse);

        /**
         * Creates a new ConnectionSetOptionDoubleResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetOptionDoubleResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetOptionDoubleResponse): database_driver_v1.ConnectionSetOptionDoubleResponse;

        /**
         * Encodes the specified ConnectionSetOptionDoubleResponse message. Does not implicitly {@link database_driver_v1.ConnectionSetOptionDoubleResponse.verify|verify} messages.
         * @param message ConnectionSetOptionDoubleResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetOptionDoubleResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetOptionDoubleResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetOptionDoubleResponse.verify|verify} messages.
         * @param message ConnectionSetOptionDoubleResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetOptionDoubleResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetOptionDoubleResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetOptionDoubleResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetOptionDoubleResponse;

        /**
         * Decodes a ConnectionSetOptionDoubleResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetOptionDoubleResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetOptionDoubleResponse;

        /**
         * Verifies a ConnectionSetOptionDoubleResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetOptionDoubleResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetOptionDoubleResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetOptionDoubleResponse;

        /**
         * Creates a plain object from a ConnectionSetOptionDoubleResponse message. Also converts values to other types if specified.
         * @param message ConnectionSetOptionDoubleResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetOptionDoubleResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetOptionDoubleResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetOptionDoubleResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionInitRequest. */
    interface IConnectionInitRequest {

        /** ConnectionInitRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionInitRequest dbHandle */
        dbHandle?: (database_driver_v1.IDatabaseHandle|null);
    }

    /** Represents a ConnectionInitRequest. */
    class ConnectionInitRequest implements IConnectionInitRequest {

        /**
         * Constructs a new ConnectionInitRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionInitRequest);

        /** ConnectionInitRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionInitRequest dbHandle. */
        public dbHandle?: (database_driver_v1.IDatabaseHandle|null);

        /**
         * Creates a new ConnectionInitRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionInitRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionInitRequest): database_driver_v1.ConnectionInitRequest;

        /**
         * Encodes the specified ConnectionInitRequest message. Does not implicitly {@link database_driver_v1.ConnectionInitRequest.verify|verify} messages.
         * @param message ConnectionInitRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionInitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionInitRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionInitRequest.verify|verify} messages.
         * @param message ConnectionInitRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionInitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionInitRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionInitRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionInitRequest;

        /**
         * Decodes a ConnectionInitRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionInitRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionInitRequest;

        /**
         * Verifies a ConnectionInitRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionInitRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionInitRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionInitRequest;

        /**
         * Creates a plain object from a ConnectionInitRequest message. Also converts values to other types if specified.
         * @param message ConnectionInitRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionInitRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionInitRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionInitRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionInitResponse. */
    interface IConnectionInitResponse {
    }

    /** Represents a ConnectionInitResponse. */
    class ConnectionInitResponse implements IConnectionInitResponse {

        /**
         * Constructs a new ConnectionInitResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionInitResponse);

        /**
         * Creates a new ConnectionInitResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionInitResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionInitResponse): database_driver_v1.ConnectionInitResponse;

        /**
         * Encodes the specified ConnectionInitResponse message. Does not implicitly {@link database_driver_v1.ConnectionInitResponse.verify|verify} messages.
         * @param message ConnectionInitResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionInitResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionInitResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionInitResponse.verify|verify} messages.
         * @param message ConnectionInitResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionInitResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionInitResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionInitResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionInitResponse;

        /**
         * Decodes a ConnectionInitResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionInitResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionInitResponse;

        /**
         * Verifies a ConnectionInitResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionInitResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionInitResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionInitResponse;

        /**
         * Creates a plain object from a ConnectionInitResponse message. Also converts values to other types if specified.
         * @param message ConnectionInitResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionInitResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionInitResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionInitResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionReleaseRequest. */
    interface IConnectionReleaseRequest {

        /** ConnectionReleaseRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);
    }

    /** Represents a ConnectionReleaseRequest. */
    class ConnectionReleaseRequest implements IConnectionReleaseRequest {

        /**
         * Constructs a new ConnectionReleaseRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionReleaseRequest);

        /** ConnectionReleaseRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /**
         * Creates a new ConnectionReleaseRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionReleaseRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionReleaseRequest): database_driver_v1.ConnectionReleaseRequest;

        /**
         * Encodes the specified ConnectionReleaseRequest message. Does not implicitly {@link database_driver_v1.ConnectionReleaseRequest.verify|verify} messages.
         * @param message ConnectionReleaseRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionReleaseRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionReleaseRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionReleaseRequest.verify|verify} messages.
         * @param message ConnectionReleaseRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionReleaseRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionReleaseRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionReleaseRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionReleaseRequest;

        /**
         * Decodes a ConnectionReleaseRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionReleaseRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionReleaseRequest;

        /**
         * Verifies a ConnectionReleaseRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionReleaseRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionReleaseRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionReleaseRequest;

        /**
         * Creates a plain object from a ConnectionReleaseRequest message. Also converts values to other types if specified.
         * @param message ConnectionReleaseRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionReleaseRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionReleaseRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionReleaseRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionReleaseResponse. */
    interface IConnectionReleaseResponse {
    }

    /** Represents a ConnectionReleaseResponse. */
    class ConnectionReleaseResponse implements IConnectionReleaseResponse {

        /**
         * Constructs a new ConnectionReleaseResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionReleaseResponse);

        /**
         * Creates a new ConnectionReleaseResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionReleaseResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionReleaseResponse): database_driver_v1.ConnectionReleaseResponse;

        /**
         * Encodes the specified ConnectionReleaseResponse message. Does not implicitly {@link database_driver_v1.ConnectionReleaseResponse.verify|verify} messages.
         * @param message ConnectionReleaseResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionReleaseResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionReleaseResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionReleaseResponse.verify|verify} messages.
         * @param message ConnectionReleaseResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionReleaseResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionReleaseResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionReleaseResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionReleaseResponse;

        /**
         * Decodes a ConnectionReleaseResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionReleaseResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionReleaseResponse;

        /**
         * Verifies a ConnectionReleaseResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionReleaseResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionReleaseResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionReleaseResponse;

        /**
         * Creates a plain object from a ConnectionReleaseResponse message. Also converts values to other types if specified.
         * @param message ConnectionReleaseResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionReleaseResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionReleaseResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionReleaseResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetInfoRequest. */
    interface IConnectionGetInfoRequest {

        /** ConnectionGetInfoRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetInfoRequest infoCodes */
        infoCodes?: (database_driver_v1.InfoCode[]|null);
    }

    /** Represents a ConnectionGetInfoRequest. */
    class ConnectionGetInfoRequest implements IConnectionGetInfoRequest {

        /**
         * Constructs a new ConnectionGetInfoRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetInfoRequest);

        /** ConnectionGetInfoRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetInfoRequest infoCodes. */
        public infoCodes: database_driver_v1.InfoCode[];

        /**
         * Creates a new ConnectionGetInfoRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetInfoRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetInfoRequest): database_driver_v1.ConnectionGetInfoRequest;

        /**
         * Encodes the specified ConnectionGetInfoRequest message. Does not implicitly {@link database_driver_v1.ConnectionGetInfoRequest.verify|verify} messages.
         * @param message ConnectionGetInfoRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetInfoRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetInfoRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetInfoRequest.verify|verify} messages.
         * @param message ConnectionGetInfoRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetInfoRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetInfoRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetInfoRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetInfoRequest;

        /**
         * Decodes a ConnectionGetInfoRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetInfoRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetInfoRequest;

        /**
         * Verifies a ConnectionGetInfoRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetInfoRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetInfoRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetInfoRequest;

        /**
         * Creates a plain object from a ConnectionGetInfoRequest message. Also converts values to other types if specified.
         * @param message ConnectionGetInfoRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetInfoRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetInfoRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetInfoRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetInfoResponse. */
    interface IConnectionGetInfoResponse {

        /** ConnectionGetInfoResponse host */
        host?: (string|null);

        /** ConnectionGetInfoResponse port */
        port?: (number|Long|null);

        /** ConnectionGetInfoResponse serverUrl */
        serverUrl?: (string|null);

        /** ConnectionGetInfoResponse sessionToken */
        sessionToken?: (string|null);

        /** ConnectionGetInfoResponse sessionId */
        sessionId?: (number|Long|null);
    }

    /** Represents a ConnectionGetInfoResponse. */
    class ConnectionGetInfoResponse implements IConnectionGetInfoResponse {

        /**
         * Constructs a new ConnectionGetInfoResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetInfoResponse);

        /** ConnectionGetInfoResponse host. */
        public host?: (string|null);

        /** ConnectionGetInfoResponse port. */
        public port?: (number|Long|null);

        /** ConnectionGetInfoResponse serverUrl. */
        public serverUrl?: (string|null);

        /** ConnectionGetInfoResponse sessionToken. */
        public sessionToken?: (string|null);

        /** ConnectionGetInfoResponse sessionId. */
        public sessionId?: (number|Long|null);

        /**
         * Creates a new ConnectionGetInfoResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetInfoResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetInfoResponse): database_driver_v1.ConnectionGetInfoResponse;

        /**
         * Encodes the specified ConnectionGetInfoResponse message. Does not implicitly {@link database_driver_v1.ConnectionGetInfoResponse.verify|verify} messages.
         * @param message ConnectionGetInfoResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetInfoResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetInfoResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetInfoResponse.verify|verify} messages.
         * @param message ConnectionGetInfoResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetInfoResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetInfoResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetInfoResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetInfoResponse;

        /**
         * Decodes a ConnectionGetInfoResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetInfoResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetInfoResponse;

        /**
         * Verifies a ConnectionGetInfoResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetInfoResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetInfoResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetInfoResponse;

        /**
         * Creates a plain object from a ConnectionGetInfoResponse message. Also converts values to other types if specified.
         * @param message ConnectionGetInfoResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetInfoResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetInfoResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetInfoResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetObjectsRequest. */
    interface IConnectionGetObjectsRequest {

        /** ConnectionGetObjectsRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetObjectsRequest depth */
        depth?: (number|null);

        /** ConnectionGetObjectsRequest catalog */
        catalog?: (string|null);

        /** ConnectionGetObjectsRequest dbSchema */
        dbSchema?: (string|null);

        /** ConnectionGetObjectsRequest tableName */
        tableName?: (string|null);

        /** ConnectionGetObjectsRequest tableType */
        tableType?: (string[]|null);

        /** ConnectionGetObjectsRequest columnName */
        columnName?: (string|null);
    }

    /** Represents a ConnectionGetObjectsRequest. */
    class ConnectionGetObjectsRequest implements IConnectionGetObjectsRequest {

        /**
         * Constructs a new ConnectionGetObjectsRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetObjectsRequest);

        /** ConnectionGetObjectsRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetObjectsRequest depth. */
        public depth: number;

        /** ConnectionGetObjectsRequest catalog. */
        public catalog?: (string|null);

        /** ConnectionGetObjectsRequest dbSchema. */
        public dbSchema?: (string|null);

        /** ConnectionGetObjectsRequest tableName. */
        public tableName?: (string|null);

        /** ConnectionGetObjectsRequest tableType. */
        public tableType: string[];

        /** ConnectionGetObjectsRequest columnName. */
        public columnName?: (string|null);

        /**
         * Creates a new ConnectionGetObjectsRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetObjectsRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetObjectsRequest): database_driver_v1.ConnectionGetObjectsRequest;

        /**
         * Encodes the specified ConnectionGetObjectsRequest message. Does not implicitly {@link database_driver_v1.ConnectionGetObjectsRequest.verify|verify} messages.
         * @param message ConnectionGetObjectsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetObjectsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetObjectsRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetObjectsRequest.verify|verify} messages.
         * @param message ConnectionGetObjectsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetObjectsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetObjectsRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetObjectsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetObjectsRequest;

        /**
         * Decodes a ConnectionGetObjectsRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetObjectsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetObjectsRequest;

        /**
         * Verifies a ConnectionGetObjectsRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetObjectsRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetObjectsRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetObjectsRequest;

        /**
         * Creates a plain object from a ConnectionGetObjectsRequest message. Also converts values to other types if specified.
         * @param message ConnectionGetObjectsRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetObjectsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetObjectsRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetObjectsRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetObjectsResponse. */
    interface IConnectionGetObjectsResponse {

        /** ConnectionGetObjectsResponse objectsData */
        objectsData?: (Uint8Array|null);
    }

    /** Represents a ConnectionGetObjectsResponse. */
    class ConnectionGetObjectsResponse implements IConnectionGetObjectsResponse {

        /**
         * Constructs a new ConnectionGetObjectsResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetObjectsResponse);

        /** ConnectionGetObjectsResponse objectsData. */
        public objectsData: Uint8Array;

        /**
         * Creates a new ConnectionGetObjectsResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetObjectsResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetObjectsResponse): database_driver_v1.ConnectionGetObjectsResponse;

        /**
         * Encodes the specified ConnectionGetObjectsResponse message. Does not implicitly {@link database_driver_v1.ConnectionGetObjectsResponse.verify|verify} messages.
         * @param message ConnectionGetObjectsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetObjectsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetObjectsResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetObjectsResponse.verify|verify} messages.
         * @param message ConnectionGetObjectsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetObjectsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetObjectsResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetObjectsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetObjectsResponse;

        /**
         * Decodes a ConnectionGetObjectsResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetObjectsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetObjectsResponse;

        /**
         * Verifies a ConnectionGetObjectsResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetObjectsResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetObjectsResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetObjectsResponse;

        /**
         * Creates a plain object from a ConnectionGetObjectsResponse message. Also converts values to other types if specified.
         * @param message ConnectionGetObjectsResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetObjectsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetObjectsResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetObjectsResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetTableSchemaRequest. */
    interface IConnectionGetTableSchemaRequest {

        /** ConnectionGetTableSchemaRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetTableSchemaRequest catalog */
        catalog?: (string|null);

        /** ConnectionGetTableSchemaRequest dbSchema */
        dbSchema?: (string|null);

        /** ConnectionGetTableSchemaRequest tableName */
        tableName?: (string|null);
    }

    /** Represents a ConnectionGetTableSchemaRequest. */
    class ConnectionGetTableSchemaRequest implements IConnectionGetTableSchemaRequest {

        /**
         * Constructs a new ConnectionGetTableSchemaRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetTableSchemaRequest);

        /** ConnectionGetTableSchemaRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetTableSchemaRequest catalog. */
        public catalog?: (string|null);

        /** ConnectionGetTableSchemaRequest dbSchema. */
        public dbSchema?: (string|null);

        /** ConnectionGetTableSchemaRequest tableName. */
        public tableName: string;

        /**
         * Creates a new ConnectionGetTableSchemaRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetTableSchemaRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetTableSchemaRequest): database_driver_v1.ConnectionGetTableSchemaRequest;

        /**
         * Encodes the specified ConnectionGetTableSchemaRequest message. Does not implicitly {@link database_driver_v1.ConnectionGetTableSchemaRequest.verify|verify} messages.
         * @param message ConnectionGetTableSchemaRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetTableSchemaRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetTableSchemaRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetTableSchemaRequest.verify|verify} messages.
         * @param message ConnectionGetTableSchemaRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetTableSchemaRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetTableSchemaRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetTableSchemaRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetTableSchemaRequest;

        /**
         * Decodes a ConnectionGetTableSchemaRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetTableSchemaRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetTableSchemaRequest;

        /**
         * Verifies a ConnectionGetTableSchemaRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetTableSchemaRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetTableSchemaRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetTableSchemaRequest;

        /**
         * Creates a plain object from a ConnectionGetTableSchemaRequest message. Also converts values to other types if specified.
         * @param message ConnectionGetTableSchemaRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetTableSchemaRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetTableSchemaRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetTableSchemaRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetTableSchemaResponse. */
    interface IConnectionGetTableSchemaResponse {

        /** ConnectionGetTableSchemaResponse schemaData */
        schemaData?: (Uint8Array|null);
    }

    /** Represents a ConnectionGetTableSchemaResponse. */
    class ConnectionGetTableSchemaResponse implements IConnectionGetTableSchemaResponse {

        /**
         * Constructs a new ConnectionGetTableSchemaResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetTableSchemaResponse);

        /** ConnectionGetTableSchemaResponse schemaData. */
        public schemaData: Uint8Array;

        /**
         * Creates a new ConnectionGetTableSchemaResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetTableSchemaResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetTableSchemaResponse): database_driver_v1.ConnectionGetTableSchemaResponse;

        /**
         * Encodes the specified ConnectionGetTableSchemaResponse message. Does not implicitly {@link database_driver_v1.ConnectionGetTableSchemaResponse.verify|verify} messages.
         * @param message ConnectionGetTableSchemaResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetTableSchemaResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetTableSchemaResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetTableSchemaResponse.verify|verify} messages.
         * @param message ConnectionGetTableSchemaResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetTableSchemaResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetTableSchemaResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetTableSchemaResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetTableSchemaResponse;

        /**
         * Decodes a ConnectionGetTableSchemaResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetTableSchemaResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetTableSchemaResponse;

        /**
         * Verifies a ConnectionGetTableSchemaResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetTableSchemaResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetTableSchemaResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetTableSchemaResponse;

        /**
         * Creates a plain object from a ConnectionGetTableSchemaResponse message. Also converts values to other types if specified.
         * @param message ConnectionGetTableSchemaResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetTableSchemaResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetTableSchemaResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetTableSchemaResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetTableTypesRequest. */
    interface IConnectionGetTableTypesRequest {

        /** ConnectionGetTableTypesRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);
    }

    /** Represents a ConnectionGetTableTypesRequest. */
    class ConnectionGetTableTypesRequest implements IConnectionGetTableTypesRequest {

        /**
         * Constructs a new ConnectionGetTableTypesRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetTableTypesRequest);

        /** ConnectionGetTableTypesRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /**
         * Creates a new ConnectionGetTableTypesRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetTableTypesRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetTableTypesRequest): database_driver_v1.ConnectionGetTableTypesRequest;

        /**
         * Encodes the specified ConnectionGetTableTypesRequest message. Does not implicitly {@link database_driver_v1.ConnectionGetTableTypesRequest.verify|verify} messages.
         * @param message ConnectionGetTableTypesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetTableTypesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetTableTypesRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetTableTypesRequest.verify|verify} messages.
         * @param message ConnectionGetTableTypesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetTableTypesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetTableTypesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetTableTypesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetTableTypesRequest;

        /**
         * Decodes a ConnectionGetTableTypesRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetTableTypesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetTableTypesRequest;

        /**
         * Verifies a ConnectionGetTableTypesRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetTableTypesRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetTableTypesRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetTableTypesRequest;

        /**
         * Creates a plain object from a ConnectionGetTableTypesRequest message. Also converts values to other types if specified.
         * @param message ConnectionGetTableTypesRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetTableTypesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetTableTypesRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetTableTypesRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetTableTypesResponse. */
    interface IConnectionGetTableTypesResponse {

        /** ConnectionGetTableTypesResponse tableTypesData */
        tableTypesData?: (Uint8Array|null);
    }

    /** Represents a ConnectionGetTableTypesResponse. */
    class ConnectionGetTableTypesResponse implements IConnectionGetTableTypesResponse {

        /**
         * Constructs a new ConnectionGetTableTypesResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetTableTypesResponse);

        /** ConnectionGetTableTypesResponse tableTypesData. */
        public tableTypesData: Uint8Array;

        /**
         * Creates a new ConnectionGetTableTypesResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetTableTypesResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetTableTypesResponse): database_driver_v1.ConnectionGetTableTypesResponse;

        /**
         * Encodes the specified ConnectionGetTableTypesResponse message. Does not implicitly {@link database_driver_v1.ConnectionGetTableTypesResponse.verify|verify} messages.
         * @param message ConnectionGetTableTypesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetTableTypesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetTableTypesResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetTableTypesResponse.verify|verify} messages.
         * @param message ConnectionGetTableTypesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetTableTypesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetTableTypesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetTableTypesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetTableTypesResponse;

        /**
         * Decodes a ConnectionGetTableTypesResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetTableTypesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetTableTypesResponse;

        /**
         * Verifies a ConnectionGetTableTypesResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetTableTypesResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetTableTypesResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetTableTypesResponse;

        /**
         * Creates a plain object from a ConnectionGetTableTypesResponse message. Also converts values to other types if specified.
         * @param message ConnectionGetTableTypesResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetTableTypesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetTableTypesResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetTableTypesResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionCommitRequest. */
    interface IConnectionCommitRequest {

        /** ConnectionCommitRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);
    }

    /** Represents a ConnectionCommitRequest. */
    class ConnectionCommitRequest implements IConnectionCommitRequest {

        /**
         * Constructs a new ConnectionCommitRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionCommitRequest);

        /** ConnectionCommitRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /**
         * Creates a new ConnectionCommitRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionCommitRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionCommitRequest): database_driver_v1.ConnectionCommitRequest;

        /**
         * Encodes the specified ConnectionCommitRequest message. Does not implicitly {@link database_driver_v1.ConnectionCommitRequest.verify|verify} messages.
         * @param message ConnectionCommitRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionCommitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionCommitRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionCommitRequest.verify|verify} messages.
         * @param message ConnectionCommitRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionCommitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionCommitRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionCommitRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionCommitRequest;

        /**
         * Decodes a ConnectionCommitRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionCommitRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionCommitRequest;

        /**
         * Verifies a ConnectionCommitRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionCommitRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionCommitRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionCommitRequest;

        /**
         * Creates a plain object from a ConnectionCommitRequest message. Also converts values to other types if specified.
         * @param message ConnectionCommitRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionCommitRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionCommitRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionCommitRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionCommitResponse. */
    interface IConnectionCommitResponse {
    }

    /** Represents a ConnectionCommitResponse. */
    class ConnectionCommitResponse implements IConnectionCommitResponse {

        /**
         * Constructs a new ConnectionCommitResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionCommitResponse);

        /**
         * Creates a new ConnectionCommitResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionCommitResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionCommitResponse): database_driver_v1.ConnectionCommitResponse;

        /**
         * Encodes the specified ConnectionCommitResponse message. Does not implicitly {@link database_driver_v1.ConnectionCommitResponse.verify|verify} messages.
         * @param message ConnectionCommitResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionCommitResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionCommitResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionCommitResponse.verify|verify} messages.
         * @param message ConnectionCommitResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionCommitResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionCommitResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionCommitResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionCommitResponse;

        /**
         * Decodes a ConnectionCommitResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionCommitResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionCommitResponse;

        /**
         * Verifies a ConnectionCommitResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionCommitResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionCommitResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionCommitResponse;

        /**
         * Creates a plain object from a ConnectionCommitResponse message. Also converts values to other types if specified.
         * @param message ConnectionCommitResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionCommitResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionCommitResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionCommitResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionRollbackRequest. */
    interface IConnectionRollbackRequest {

        /** ConnectionRollbackRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);
    }

    /** Represents a ConnectionRollbackRequest. */
    class ConnectionRollbackRequest implements IConnectionRollbackRequest {

        /**
         * Constructs a new ConnectionRollbackRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionRollbackRequest);

        /** ConnectionRollbackRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /**
         * Creates a new ConnectionRollbackRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionRollbackRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionRollbackRequest): database_driver_v1.ConnectionRollbackRequest;

        /**
         * Encodes the specified ConnectionRollbackRequest message. Does not implicitly {@link database_driver_v1.ConnectionRollbackRequest.verify|verify} messages.
         * @param message ConnectionRollbackRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionRollbackRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionRollbackRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionRollbackRequest.verify|verify} messages.
         * @param message ConnectionRollbackRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionRollbackRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionRollbackRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionRollbackRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionRollbackRequest;

        /**
         * Decodes a ConnectionRollbackRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionRollbackRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionRollbackRequest;

        /**
         * Verifies a ConnectionRollbackRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionRollbackRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionRollbackRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionRollbackRequest;

        /**
         * Creates a plain object from a ConnectionRollbackRequest message. Also converts values to other types if specified.
         * @param message ConnectionRollbackRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionRollbackRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionRollbackRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionRollbackRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionRollbackResponse. */
    interface IConnectionRollbackResponse {
    }

    /** Represents a ConnectionRollbackResponse. */
    class ConnectionRollbackResponse implements IConnectionRollbackResponse {

        /**
         * Constructs a new ConnectionRollbackResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionRollbackResponse);

        /**
         * Creates a new ConnectionRollbackResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionRollbackResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionRollbackResponse): database_driver_v1.ConnectionRollbackResponse;

        /**
         * Encodes the specified ConnectionRollbackResponse message. Does not implicitly {@link database_driver_v1.ConnectionRollbackResponse.verify|verify} messages.
         * @param message ConnectionRollbackResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionRollbackResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionRollbackResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionRollbackResponse.verify|verify} messages.
         * @param message ConnectionRollbackResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionRollbackResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionRollbackResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionRollbackResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionRollbackResponse;

        /**
         * Decodes a ConnectionRollbackResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionRollbackResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionRollbackResponse;

        /**
         * Verifies a ConnectionRollbackResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionRollbackResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionRollbackResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionRollbackResponse;

        /**
         * Creates a plain object from a ConnectionRollbackResponse message. Also converts values to other types if specified.
         * @param message ConnectionRollbackResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionRollbackResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionRollbackResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionRollbackResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetSessionParametersRequest. */
    interface IConnectionSetSessionParametersRequest {

        /** ConnectionSetSessionParametersRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetSessionParametersRequest parameters */
        parameters?: ({ [k: string]: string }|null);
    }

    /** Represents a ConnectionSetSessionParametersRequest. */
    class ConnectionSetSessionParametersRequest implements IConnectionSetSessionParametersRequest {

        /**
         * Constructs a new ConnectionSetSessionParametersRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetSessionParametersRequest);

        /** ConnectionSetSessionParametersRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionSetSessionParametersRequest parameters. */
        public parameters: { [k: string]: string };

        /**
         * Creates a new ConnectionSetSessionParametersRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetSessionParametersRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetSessionParametersRequest): database_driver_v1.ConnectionSetSessionParametersRequest;

        /**
         * Encodes the specified ConnectionSetSessionParametersRequest message. Does not implicitly {@link database_driver_v1.ConnectionSetSessionParametersRequest.verify|verify} messages.
         * @param message ConnectionSetSessionParametersRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetSessionParametersRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetSessionParametersRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetSessionParametersRequest.verify|verify} messages.
         * @param message ConnectionSetSessionParametersRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetSessionParametersRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetSessionParametersRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetSessionParametersRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetSessionParametersRequest;

        /**
         * Decodes a ConnectionSetSessionParametersRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetSessionParametersRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetSessionParametersRequest;

        /**
         * Verifies a ConnectionSetSessionParametersRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetSessionParametersRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetSessionParametersRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetSessionParametersRequest;

        /**
         * Creates a plain object from a ConnectionSetSessionParametersRequest message. Also converts values to other types if specified.
         * @param message ConnectionSetSessionParametersRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetSessionParametersRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetSessionParametersRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetSessionParametersRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionSetSessionParametersResponse. */
    interface IConnectionSetSessionParametersResponse {
    }

    /** Represents a ConnectionSetSessionParametersResponse. */
    class ConnectionSetSessionParametersResponse implements IConnectionSetSessionParametersResponse {

        /**
         * Constructs a new ConnectionSetSessionParametersResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionSetSessionParametersResponse);

        /**
         * Creates a new ConnectionSetSessionParametersResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionSetSessionParametersResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionSetSessionParametersResponse): database_driver_v1.ConnectionSetSessionParametersResponse;

        /**
         * Encodes the specified ConnectionSetSessionParametersResponse message. Does not implicitly {@link database_driver_v1.ConnectionSetSessionParametersResponse.verify|verify} messages.
         * @param message ConnectionSetSessionParametersResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionSetSessionParametersResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionSetSessionParametersResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionSetSessionParametersResponse.verify|verify} messages.
         * @param message ConnectionSetSessionParametersResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionSetSessionParametersResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionSetSessionParametersResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionSetSessionParametersResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionSetSessionParametersResponse;

        /**
         * Decodes a ConnectionSetSessionParametersResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionSetSessionParametersResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionSetSessionParametersResponse;

        /**
         * Verifies a ConnectionSetSessionParametersResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionSetSessionParametersResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionSetSessionParametersResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionSetSessionParametersResponse;

        /**
         * Creates a plain object from a ConnectionSetSessionParametersResponse message. Also converts values to other types if specified.
         * @param message ConnectionSetSessionParametersResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionSetSessionParametersResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionSetSessionParametersResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionSetSessionParametersResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetParameterRequest. */
    interface IConnectionGetParameterRequest {

        /** ConnectionGetParameterRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetParameterRequest key */
        key?: (string|null);
    }

    /** Represents a ConnectionGetParameterRequest. */
    class ConnectionGetParameterRequest implements IConnectionGetParameterRequest {

        /**
         * Constructs a new ConnectionGetParameterRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetParameterRequest);

        /** ConnectionGetParameterRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /** ConnectionGetParameterRequest key. */
        public key: string;

        /**
         * Creates a new ConnectionGetParameterRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetParameterRequest instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetParameterRequest): database_driver_v1.ConnectionGetParameterRequest;

        /**
         * Encodes the specified ConnectionGetParameterRequest message. Does not implicitly {@link database_driver_v1.ConnectionGetParameterRequest.verify|verify} messages.
         * @param message ConnectionGetParameterRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetParameterRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetParameterRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetParameterRequest.verify|verify} messages.
         * @param message ConnectionGetParameterRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetParameterRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetParameterRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetParameterRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetParameterRequest;

        /**
         * Decodes a ConnectionGetParameterRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetParameterRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetParameterRequest;

        /**
         * Verifies a ConnectionGetParameterRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetParameterRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetParameterRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetParameterRequest;

        /**
         * Creates a plain object from a ConnectionGetParameterRequest message. Also converts values to other types if specified.
         * @param message ConnectionGetParameterRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetParameterRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetParameterRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetParameterRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConnectionGetParameterResponse. */
    interface IConnectionGetParameterResponse {

        /** ConnectionGetParameterResponse value */
        value?: (string|null);
    }

    /** Represents a ConnectionGetParameterResponse. */
    class ConnectionGetParameterResponse implements IConnectionGetParameterResponse {

        /**
         * Constructs a new ConnectionGetParameterResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConnectionGetParameterResponse);

        /** ConnectionGetParameterResponse value. */
        public value?: (string|null);

        /**
         * Creates a new ConnectionGetParameterResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConnectionGetParameterResponse instance
         */
        public static create(properties?: database_driver_v1.IConnectionGetParameterResponse): database_driver_v1.ConnectionGetParameterResponse;

        /**
         * Encodes the specified ConnectionGetParameterResponse message. Does not implicitly {@link database_driver_v1.ConnectionGetParameterResponse.verify|verify} messages.
         * @param message ConnectionGetParameterResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConnectionGetParameterResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConnectionGetParameterResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConnectionGetParameterResponse.verify|verify} messages.
         * @param message ConnectionGetParameterResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConnectionGetParameterResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConnectionGetParameterResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConnectionGetParameterResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConnectionGetParameterResponse;

        /**
         * Decodes a ConnectionGetParameterResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConnectionGetParameterResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConnectionGetParameterResponse;

        /**
         * Verifies a ConnectionGetParameterResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConnectionGetParameterResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConnectionGetParameterResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConnectionGetParameterResponse;

        /**
         * Creates a plain object from a ConnectionGetParameterResponse message. Also converts values to other types if specified.
         * @param message ConnectionGetParameterResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConnectionGetParameterResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConnectionGetParameterResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConnectionGetParameterResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementNewRequest. */
    interface IStatementNewRequest {

        /** StatementNewRequest connHandle */
        connHandle?: (database_driver_v1.IConnectionHandle|null);
    }

    /** Represents a StatementNewRequest. */
    class StatementNewRequest implements IStatementNewRequest {

        /**
         * Constructs a new StatementNewRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementNewRequest);

        /** StatementNewRequest connHandle. */
        public connHandle?: (database_driver_v1.IConnectionHandle|null);

        /**
         * Creates a new StatementNewRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementNewRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementNewRequest): database_driver_v1.StatementNewRequest;

        /**
         * Encodes the specified StatementNewRequest message. Does not implicitly {@link database_driver_v1.StatementNewRequest.verify|verify} messages.
         * @param message StatementNewRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementNewRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementNewRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementNewRequest.verify|verify} messages.
         * @param message StatementNewRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementNewRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementNewRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementNewRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementNewRequest;

        /**
         * Decodes a StatementNewRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementNewRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementNewRequest;

        /**
         * Verifies a StatementNewRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementNewRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementNewRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementNewRequest;

        /**
         * Creates a plain object from a StatementNewRequest message. Also converts values to other types if specified.
         * @param message StatementNewRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementNewRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementNewRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementNewRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementNewResponse. */
    interface IStatementNewResponse {

        /** StatementNewResponse stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);
    }

    /** Represents a StatementNewResponse. */
    class StatementNewResponse implements IStatementNewResponse {

        /**
         * Constructs a new StatementNewResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementNewResponse);

        /** StatementNewResponse stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /**
         * Creates a new StatementNewResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementNewResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementNewResponse): database_driver_v1.StatementNewResponse;

        /**
         * Encodes the specified StatementNewResponse message. Does not implicitly {@link database_driver_v1.StatementNewResponse.verify|verify} messages.
         * @param message StatementNewResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementNewResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementNewResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementNewResponse.verify|verify} messages.
         * @param message StatementNewResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementNewResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementNewResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementNewResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementNewResponse;

        /**
         * Decodes a StatementNewResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementNewResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementNewResponse;

        /**
         * Verifies a StatementNewResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementNewResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementNewResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementNewResponse;

        /**
         * Creates a plain object from a StatementNewResponse message. Also converts values to other types if specified.
         * @param message StatementNewResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementNewResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementNewResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementNewResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementReleaseRequest. */
    interface IStatementReleaseRequest {

        /** StatementReleaseRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);
    }

    /** Represents a StatementReleaseRequest. */
    class StatementReleaseRequest implements IStatementReleaseRequest {

        /**
         * Constructs a new StatementReleaseRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementReleaseRequest);

        /** StatementReleaseRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /**
         * Creates a new StatementReleaseRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementReleaseRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementReleaseRequest): database_driver_v1.StatementReleaseRequest;

        /**
         * Encodes the specified StatementReleaseRequest message. Does not implicitly {@link database_driver_v1.StatementReleaseRequest.verify|verify} messages.
         * @param message StatementReleaseRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementReleaseRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementReleaseRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementReleaseRequest.verify|verify} messages.
         * @param message StatementReleaseRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementReleaseRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementReleaseRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementReleaseRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementReleaseRequest;

        /**
         * Decodes a StatementReleaseRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementReleaseRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementReleaseRequest;

        /**
         * Verifies a StatementReleaseRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementReleaseRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementReleaseRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementReleaseRequest;

        /**
         * Creates a plain object from a StatementReleaseRequest message. Also converts values to other types if specified.
         * @param message StatementReleaseRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementReleaseRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementReleaseRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementReleaseRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementReleaseResponse. */
    interface IStatementReleaseResponse {
    }

    /** Represents a StatementReleaseResponse. */
    class StatementReleaseResponse implements IStatementReleaseResponse {

        /**
         * Constructs a new StatementReleaseResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementReleaseResponse);

        /**
         * Creates a new StatementReleaseResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementReleaseResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementReleaseResponse): database_driver_v1.StatementReleaseResponse;

        /**
         * Encodes the specified StatementReleaseResponse message. Does not implicitly {@link database_driver_v1.StatementReleaseResponse.verify|verify} messages.
         * @param message StatementReleaseResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementReleaseResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementReleaseResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementReleaseResponse.verify|verify} messages.
         * @param message StatementReleaseResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementReleaseResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementReleaseResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementReleaseResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementReleaseResponse;

        /**
         * Decodes a StatementReleaseResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementReleaseResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementReleaseResponse;

        /**
         * Verifies a StatementReleaseResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementReleaseResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementReleaseResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementReleaseResponse;

        /**
         * Creates a plain object from a StatementReleaseResponse message. Also converts values to other types if specified.
         * @param message StatementReleaseResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementReleaseResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementReleaseResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementReleaseResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetSqlQueryRequest. */
    interface IStatementSetSqlQueryRequest {

        /** StatementSetSqlQueryRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetSqlQueryRequest query */
        query?: (string|null);
    }

    /** Represents a StatementSetSqlQueryRequest. */
    class StatementSetSqlQueryRequest implements IStatementSetSqlQueryRequest {

        /**
         * Constructs a new StatementSetSqlQueryRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetSqlQueryRequest);

        /** StatementSetSqlQueryRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetSqlQueryRequest query. */
        public query: string;

        /**
         * Creates a new StatementSetSqlQueryRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetSqlQueryRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementSetSqlQueryRequest): database_driver_v1.StatementSetSqlQueryRequest;

        /**
         * Encodes the specified StatementSetSqlQueryRequest message. Does not implicitly {@link database_driver_v1.StatementSetSqlQueryRequest.verify|verify} messages.
         * @param message StatementSetSqlQueryRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetSqlQueryRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetSqlQueryRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetSqlQueryRequest.verify|verify} messages.
         * @param message StatementSetSqlQueryRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetSqlQueryRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetSqlQueryRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetSqlQueryRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetSqlQueryRequest;

        /**
         * Decodes a StatementSetSqlQueryRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetSqlQueryRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetSqlQueryRequest;

        /**
         * Verifies a StatementSetSqlQueryRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetSqlQueryRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetSqlQueryRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetSqlQueryRequest;

        /**
         * Creates a plain object from a StatementSetSqlQueryRequest message. Also converts values to other types if specified.
         * @param message StatementSetSqlQueryRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetSqlQueryRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetSqlQueryRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetSqlQueryRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetSqlQueryResponse. */
    interface IStatementSetSqlQueryResponse {
    }

    /** Represents a StatementSetSqlQueryResponse. */
    class StatementSetSqlQueryResponse implements IStatementSetSqlQueryResponse {

        /**
         * Constructs a new StatementSetSqlQueryResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetSqlQueryResponse);

        /**
         * Creates a new StatementSetSqlQueryResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetSqlQueryResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementSetSqlQueryResponse): database_driver_v1.StatementSetSqlQueryResponse;

        /**
         * Encodes the specified StatementSetSqlQueryResponse message. Does not implicitly {@link database_driver_v1.StatementSetSqlQueryResponse.verify|verify} messages.
         * @param message StatementSetSqlQueryResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetSqlQueryResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetSqlQueryResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetSqlQueryResponse.verify|verify} messages.
         * @param message StatementSetSqlQueryResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetSqlQueryResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetSqlQueryResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetSqlQueryResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetSqlQueryResponse;

        /**
         * Decodes a StatementSetSqlQueryResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetSqlQueryResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetSqlQueryResponse;

        /**
         * Verifies a StatementSetSqlQueryResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetSqlQueryResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetSqlQueryResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetSqlQueryResponse;

        /**
         * Creates a plain object from a StatementSetSqlQueryResponse message. Also converts values to other types if specified.
         * @param message StatementSetSqlQueryResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetSqlQueryResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetSqlQueryResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetSqlQueryResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetSubstraitPlanRequest. */
    interface IStatementSetSubstraitPlanRequest {

        /** StatementSetSubstraitPlanRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetSubstraitPlanRequest plan */
        plan?: (Uint8Array|null);
    }

    /** Represents a StatementSetSubstraitPlanRequest. */
    class StatementSetSubstraitPlanRequest implements IStatementSetSubstraitPlanRequest {

        /**
         * Constructs a new StatementSetSubstraitPlanRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetSubstraitPlanRequest);

        /** StatementSetSubstraitPlanRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetSubstraitPlanRequest plan. */
        public plan: Uint8Array;

        /**
         * Creates a new StatementSetSubstraitPlanRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetSubstraitPlanRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementSetSubstraitPlanRequest): database_driver_v1.StatementSetSubstraitPlanRequest;

        /**
         * Encodes the specified StatementSetSubstraitPlanRequest message. Does not implicitly {@link database_driver_v1.StatementSetSubstraitPlanRequest.verify|verify} messages.
         * @param message StatementSetSubstraitPlanRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetSubstraitPlanRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetSubstraitPlanRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetSubstraitPlanRequest.verify|verify} messages.
         * @param message StatementSetSubstraitPlanRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetSubstraitPlanRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetSubstraitPlanRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetSubstraitPlanRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetSubstraitPlanRequest;

        /**
         * Decodes a StatementSetSubstraitPlanRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetSubstraitPlanRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetSubstraitPlanRequest;

        /**
         * Verifies a StatementSetSubstraitPlanRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetSubstraitPlanRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetSubstraitPlanRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetSubstraitPlanRequest;

        /**
         * Creates a plain object from a StatementSetSubstraitPlanRequest message. Also converts values to other types if specified.
         * @param message StatementSetSubstraitPlanRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetSubstraitPlanRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetSubstraitPlanRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetSubstraitPlanRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetSubstraitPlanResponse. */
    interface IStatementSetSubstraitPlanResponse {
    }

    /** Represents a StatementSetSubstraitPlanResponse. */
    class StatementSetSubstraitPlanResponse implements IStatementSetSubstraitPlanResponse {

        /**
         * Constructs a new StatementSetSubstraitPlanResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetSubstraitPlanResponse);

        /**
         * Creates a new StatementSetSubstraitPlanResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetSubstraitPlanResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementSetSubstraitPlanResponse): database_driver_v1.StatementSetSubstraitPlanResponse;

        /**
         * Encodes the specified StatementSetSubstraitPlanResponse message. Does not implicitly {@link database_driver_v1.StatementSetSubstraitPlanResponse.verify|verify} messages.
         * @param message StatementSetSubstraitPlanResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetSubstraitPlanResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetSubstraitPlanResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetSubstraitPlanResponse.verify|verify} messages.
         * @param message StatementSetSubstraitPlanResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetSubstraitPlanResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetSubstraitPlanResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetSubstraitPlanResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetSubstraitPlanResponse;

        /**
         * Decodes a StatementSetSubstraitPlanResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetSubstraitPlanResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetSubstraitPlanResponse;

        /**
         * Verifies a StatementSetSubstraitPlanResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetSubstraitPlanResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetSubstraitPlanResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetSubstraitPlanResponse;

        /**
         * Creates a plain object from a StatementSetSubstraitPlanResponse message. Also converts values to other types if specified.
         * @param message StatementSetSubstraitPlanResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetSubstraitPlanResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetSubstraitPlanResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetSubstraitPlanResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a PrepareResult. */
    interface IPrepareResult {

        /** PrepareResult stream */
        stream?: (database_driver_v1.IArrowArrayStreamPtr|null);

        /** PrepareResult columns */
        columns?: (database_driver_v1.IColumnMetadata[]|null);
    }

    /** Represents a PrepareResult. */
    class PrepareResult implements IPrepareResult {

        /**
         * Constructs a new PrepareResult.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IPrepareResult);

        /** PrepareResult stream. */
        public stream?: (database_driver_v1.IArrowArrayStreamPtr|null);

        /** PrepareResult columns. */
        public columns: database_driver_v1.IColumnMetadata[];

        /**
         * Creates a new PrepareResult instance using the specified properties.
         * @param [properties] Properties to set
         * @returns PrepareResult instance
         */
        public static create(properties?: database_driver_v1.IPrepareResult): database_driver_v1.PrepareResult;

        /**
         * Encodes the specified PrepareResult message. Does not implicitly {@link database_driver_v1.PrepareResult.verify|verify} messages.
         * @param message PrepareResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IPrepareResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified PrepareResult message, length delimited. Does not implicitly {@link database_driver_v1.PrepareResult.verify|verify} messages.
         * @param message PrepareResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IPrepareResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a PrepareResult message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns PrepareResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.PrepareResult;

        /**
         * Decodes a PrepareResult message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns PrepareResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.PrepareResult;

        /**
         * Verifies a PrepareResult message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a PrepareResult message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns PrepareResult
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.PrepareResult;

        /**
         * Creates a plain object from a PrepareResult message. Also converts values to other types if specified.
         * @param message PrepareResult
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.PrepareResult, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this PrepareResult to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for PrepareResult
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementPrepareRequest. */
    interface IStatementPrepareRequest {

        /** StatementPrepareRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);
    }

    /** Represents a StatementPrepareRequest. */
    class StatementPrepareRequest implements IStatementPrepareRequest {

        /**
         * Constructs a new StatementPrepareRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementPrepareRequest);

        /** StatementPrepareRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /**
         * Creates a new StatementPrepareRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementPrepareRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementPrepareRequest): database_driver_v1.StatementPrepareRequest;

        /**
         * Encodes the specified StatementPrepareRequest message. Does not implicitly {@link database_driver_v1.StatementPrepareRequest.verify|verify} messages.
         * @param message StatementPrepareRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementPrepareRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementPrepareRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementPrepareRequest.verify|verify} messages.
         * @param message StatementPrepareRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementPrepareRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementPrepareRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementPrepareRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementPrepareRequest;

        /**
         * Decodes a StatementPrepareRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementPrepareRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementPrepareRequest;

        /**
         * Verifies a StatementPrepareRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementPrepareRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementPrepareRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementPrepareRequest;

        /**
         * Creates a plain object from a StatementPrepareRequest message. Also converts values to other types if specified.
         * @param message StatementPrepareRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementPrepareRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementPrepareRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementPrepareRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementPrepareResponse. */
    interface IStatementPrepareResponse {

        /** StatementPrepareResponse result */
        result?: (database_driver_v1.IPrepareResult|null);
    }

    /** Represents a StatementPrepareResponse. */
    class StatementPrepareResponse implements IStatementPrepareResponse {

        /**
         * Constructs a new StatementPrepareResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementPrepareResponse);

        /** StatementPrepareResponse result. */
        public result?: (database_driver_v1.IPrepareResult|null);

        /**
         * Creates a new StatementPrepareResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementPrepareResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementPrepareResponse): database_driver_v1.StatementPrepareResponse;

        /**
         * Encodes the specified StatementPrepareResponse message. Does not implicitly {@link database_driver_v1.StatementPrepareResponse.verify|verify} messages.
         * @param message StatementPrepareResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementPrepareResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementPrepareResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementPrepareResponse.verify|verify} messages.
         * @param message StatementPrepareResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementPrepareResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementPrepareResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementPrepareResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementPrepareResponse;

        /**
         * Decodes a StatementPrepareResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementPrepareResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementPrepareResponse;

        /**
         * Verifies a StatementPrepareResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementPrepareResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementPrepareResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementPrepareResponse;

        /**
         * Creates a plain object from a StatementPrepareResponse message. Also converts values to other types if specified.
         * @param message StatementPrepareResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementPrepareResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementPrepareResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementPrepareResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionStringRequest. */
    interface IStatementSetOptionStringRequest {

        /** StatementSetOptionStringRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionStringRequest key */
        key?: (string|null);

        /** StatementSetOptionStringRequest value */
        value?: (string|null);
    }

    /** Represents a StatementSetOptionStringRequest. */
    class StatementSetOptionStringRequest implements IStatementSetOptionStringRequest {

        /**
         * Constructs a new StatementSetOptionStringRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionStringRequest);

        /** StatementSetOptionStringRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionStringRequest key. */
        public key: string;

        /** StatementSetOptionStringRequest value. */
        public value: string;

        /**
         * Creates a new StatementSetOptionStringRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionStringRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionStringRequest): database_driver_v1.StatementSetOptionStringRequest;

        /**
         * Encodes the specified StatementSetOptionStringRequest message. Does not implicitly {@link database_driver_v1.StatementSetOptionStringRequest.verify|verify} messages.
         * @param message StatementSetOptionStringRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionStringRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionStringRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionStringRequest.verify|verify} messages.
         * @param message StatementSetOptionStringRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionStringRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionStringRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionStringRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionStringRequest;

        /**
         * Decodes a StatementSetOptionStringRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionStringRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionStringRequest;

        /**
         * Verifies a StatementSetOptionStringRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionStringRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionStringRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionStringRequest;

        /**
         * Creates a plain object from a StatementSetOptionStringRequest message. Also converts values to other types if specified.
         * @param message StatementSetOptionStringRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionStringRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionStringRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionStringRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionStringResponse. */
    interface IStatementSetOptionStringResponse {
    }

    /** Represents a StatementSetOptionStringResponse. */
    class StatementSetOptionStringResponse implements IStatementSetOptionStringResponse {

        /**
         * Constructs a new StatementSetOptionStringResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionStringResponse);

        /**
         * Creates a new StatementSetOptionStringResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionStringResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionStringResponse): database_driver_v1.StatementSetOptionStringResponse;

        /**
         * Encodes the specified StatementSetOptionStringResponse message. Does not implicitly {@link database_driver_v1.StatementSetOptionStringResponse.verify|verify} messages.
         * @param message StatementSetOptionStringResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionStringResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionStringResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionStringResponse.verify|verify} messages.
         * @param message StatementSetOptionStringResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionStringResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionStringResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionStringResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionStringResponse;

        /**
         * Decodes a StatementSetOptionStringResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionStringResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionStringResponse;

        /**
         * Verifies a StatementSetOptionStringResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionStringResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionStringResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionStringResponse;

        /**
         * Creates a plain object from a StatementSetOptionStringResponse message. Also converts values to other types if specified.
         * @param message StatementSetOptionStringResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionStringResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionStringResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionStringResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionBytesRequest. */
    interface IStatementSetOptionBytesRequest {

        /** StatementSetOptionBytesRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionBytesRequest key */
        key?: (string|null);

        /** StatementSetOptionBytesRequest value */
        value?: (Uint8Array|null);
    }

    /** Represents a StatementSetOptionBytesRequest. */
    class StatementSetOptionBytesRequest implements IStatementSetOptionBytesRequest {

        /**
         * Constructs a new StatementSetOptionBytesRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionBytesRequest);

        /** StatementSetOptionBytesRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionBytesRequest key. */
        public key: string;

        /** StatementSetOptionBytesRequest value. */
        public value: Uint8Array;

        /**
         * Creates a new StatementSetOptionBytesRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionBytesRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionBytesRequest): database_driver_v1.StatementSetOptionBytesRequest;

        /**
         * Encodes the specified StatementSetOptionBytesRequest message. Does not implicitly {@link database_driver_v1.StatementSetOptionBytesRequest.verify|verify} messages.
         * @param message StatementSetOptionBytesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionBytesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionBytesRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionBytesRequest.verify|verify} messages.
         * @param message StatementSetOptionBytesRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionBytesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionBytesRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionBytesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionBytesRequest;

        /**
         * Decodes a StatementSetOptionBytesRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionBytesRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionBytesRequest;

        /**
         * Verifies a StatementSetOptionBytesRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionBytesRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionBytesRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionBytesRequest;

        /**
         * Creates a plain object from a StatementSetOptionBytesRequest message. Also converts values to other types if specified.
         * @param message StatementSetOptionBytesRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionBytesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionBytesRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionBytesRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionBytesResponse. */
    interface IStatementSetOptionBytesResponse {
    }

    /** Represents a StatementSetOptionBytesResponse. */
    class StatementSetOptionBytesResponse implements IStatementSetOptionBytesResponse {

        /**
         * Constructs a new StatementSetOptionBytesResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionBytesResponse);

        /**
         * Creates a new StatementSetOptionBytesResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionBytesResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionBytesResponse): database_driver_v1.StatementSetOptionBytesResponse;

        /**
         * Encodes the specified StatementSetOptionBytesResponse message. Does not implicitly {@link database_driver_v1.StatementSetOptionBytesResponse.verify|verify} messages.
         * @param message StatementSetOptionBytesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionBytesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionBytesResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionBytesResponse.verify|verify} messages.
         * @param message StatementSetOptionBytesResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionBytesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionBytesResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionBytesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionBytesResponse;

        /**
         * Decodes a StatementSetOptionBytesResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionBytesResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionBytesResponse;

        /**
         * Verifies a StatementSetOptionBytesResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionBytesResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionBytesResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionBytesResponse;

        /**
         * Creates a plain object from a StatementSetOptionBytesResponse message. Also converts values to other types if specified.
         * @param message StatementSetOptionBytesResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionBytesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionBytesResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionBytesResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionIntRequest. */
    interface IStatementSetOptionIntRequest {

        /** StatementSetOptionIntRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionIntRequest key */
        key?: (string|null);

        /** StatementSetOptionIntRequest value */
        value?: (number|Long|null);
    }

    /** Represents a StatementSetOptionIntRequest. */
    class StatementSetOptionIntRequest implements IStatementSetOptionIntRequest {

        /**
         * Constructs a new StatementSetOptionIntRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionIntRequest);

        /** StatementSetOptionIntRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionIntRequest key. */
        public key: string;

        /** StatementSetOptionIntRequest value. */
        public value: (number|Long);

        /**
         * Creates a new StatementSetOptionIntRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionIntRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionIntRequest): database_driver_v1.StatementSetOptionIntRequest;

        /**
         * Encodes the specified StatementSetOptionIntRequest message. Does not implicitly {@link database_driver_v1.StatementSetOptionIntRequest.verify|verify} messages.
         * @param message StatementSetOptionIntRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionIntRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionIntRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionIntRequest.verify|verify} messages.
         * @param message StatementSetOptionIntRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionIntRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionIntRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionIntRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionIntRequest;

        /**
         * Decodes a StatementSetOptionIntRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionIntRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionIntRequest;

        /**
         * Verifies a StatementSetOptionIntRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionIntRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionIntRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionIntRequest;

        /**
         * Creates a plain object from a StatementSetOptionIntRequest message. Also converts values to other types if specified.
         * @param message StatementSetOptionIntRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionIntRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionIntRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionIntRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionIntResponse. */
    interface IStatementSetOptionIntResponse {
    }

    /** Represents a StatementSetOptionIntResponse. */
    class StatementSetOptionIntResponse implements IStatementSetOptionIntResponse {

        /**
         * Constructs a new StatementSetOptionIntResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionIntResponse);

        /**
         * Creates a new StatementSetOptionIntResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionIntResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionIntResponse): database_driver_v1.StatementSetOptionIntResponse;

        /**
         * Encodes the specified StatementSetOptionIntResponse message. Does not implicitly {@link database_driver_v1.StatementSetOptionIntResponse.verify|verify} messages.
         * @param message StatementSetOptionIntResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionIntResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionIntResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionIntResponse.verify|verify} messages.
         * @param message StatementSetOptionIntResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionIntResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionIntResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionIntResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionIntResponse;

        /**
         * Decodes a StatementSetOptionIntResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionIntResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionIntResponse;

        /**
         * Verifies a StatementSetOptionIntResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionIntResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionIntResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionIntResponse;

        /**
         * Creates a plain object from a StatementSetOptionIntResponse message. Also converts values to other types if specified.
         * @param message StatementSetOptionIntResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionIntResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionIntResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionIntResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionDoubleRequest. */
    interface IStatementSetOptionDoubleRequest {

        /** StatementSetOptionDoubleRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionDoubleRequest key */
        key?: (string|null);

        /** StatementSetOptionDoubleRequest value */
        value?: (number|null);
    }

    /** Represents a StatementSetOptionDoubleRequest. */
    class StatementSetOptionDoubleRequest implements IStatementSetOptionDoubleRequest {

        /**
         * Constructs a new StatementSetOptionDoubleRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionDoubleRequest);

        /** StatementSetOptionDoubleRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementSetOptionDoubleRequest key. */
        public key: string;

        /** StatementSetOptionDoubleRequest value. */
        public value: number;

        /**
         * Creates a new StatementSetOptionDoubleRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionDoubleRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionDoubleRequest): database_driver_v1.StatementSetOptionDoubleRequest;

        /**
         * Encodes the specified StatementSetOptionDoubleRequest message. Does not implicitly {@link database_driver_v1.StatementSetOptionDoubleRequest.verify|verify} messages.
         * @param message StatementSetOptionDoubleRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionDoubleRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionDoubleRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionDoubleRequest.verify|verify} messages.
         * @param message StatementSetOptionDoubleRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionDoubleRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionDoubleRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionDoubleRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionDoubleRequest;

        /**
         * Decodes a StatementSetOptionDoubleRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionDoubleRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionDoubleRequest;

        /**
         * Verifies a StatementSetOptionDoubleRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionDoubleRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionDoubleRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionDoubleRequest;

        /**
         * Creates a plain object from a StatementSetOptionDoubleRequest message. Also converts values to other types if specified.
         * @param message StatementSetOptionDoubleRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionDoubleRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionDoubleRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionDoubleRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementSetOptionDoubleResponse. */
    interface IStatementSetOptionDoubleResponse {
    }

    /** Represents a StatementSetOptionDoubleResponse. */
    class StatementSetOptionDoubleResponse implements IStatementSetOptionDoubleResponse {

        /**
         * Constructs a new StatementSetOptionDoubleResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementSetOptionDoubleResponse);

        /**
         * Creates a new StatementSetOptionDoubleResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementSetOptionDoubleResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementSetOptionDoubleResponse): database_driver_v1.StatementSetOptionDoubleResponse;

        /**
         * Encodes the specified StatementSetOptionDoubleResponse message. Does not implicitly {@link database_driver_v1.StatementSetOptionDoubleResponse.verify|verify} messages.
         * @param message StatementSetOptionDoubleResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementSetOptionDoubleResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementSetOptionDoubleResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementSetOptionDoubleResponse.verify|verify} messages.
         * @param message StatementSetOptionDoubleResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementSetOptionDoubleResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementSetOptionDoubleResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementSetOptionDoubleResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementSetOptionDoubleResponse;

        /**
         * Decodes a StatementSetOptionDoubleResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementSetOptionDoubleResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementSetOptionDoubleResponse;

        /**
         * Verifies a StatementSetOptionDoubleResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementSetOptionDoubleResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementSetOptionDoubleResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementSetOptionDoubleResponse;

        /**
         * Creates a plain object from a StatementSetOptionDoubleResponse message. Also converts values to other types if specified.
         * @param message StatementSetOptionDoubleResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementSetOptionDoubleResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementSetOptionDoubleResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementSetOptionDoubleResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementGetParameterSchemaRequest. */
    interface IStatementGetParameterSchemaRequest {

        /** StatementGetParameterSchemaRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);
    }

    /** Represents a StatementGetParameterSchemaRequest. */
    class StatementGetParameterSchemaRequest implements IStatementGetParameterSchemaRequest {

        /**
         * Constructs a new StatementGetParameterSchemaRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementGetParameterSchemaRequest);

        /** StatementGetParameterSchemaRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /**
         * Creates a new StatementGetParameterSchemaRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementGetParameterSchemaRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementGetParameterSchemaRequest): database_driver_v1.StatementGetParameterSchemaRequest;

        /**
         * Encodes the specified StatementGetParameterSchemaRequest message. Does not implicitly {@link database_driver_v1.StatementGetParameterSchemaRequest.verify|verify} messages.
         * @param message StatementGetParameterSchemaRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementGetParameterSchemaRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementGetParameterSchemaRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementGetParameterSchemaRequest.verify|verify} messages.
         * @param message StatementGetParameterSchemaRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementGetParameterSchemaRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementGetParameterSchemaRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementGetParameterSchemaRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementGetParameterSchemaRequest;

        /**
         * Decodes a StatementGetParameterSchemaRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementGetParameterSchemaRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementGetParameterSchemaRequest;

        /**
         * Verifies a StatementGetParameterSchemaRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementGetParameterSchemaRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementGetParameterSchemaRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementGetParameterSchemaRequest;

        /**
         * Creates a plain object from a StatementGetParameterSchemaRequest message. Also converts values to other types if specified.
         * @param message StatementGetParameterSchemaRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementGetParameterSchemaRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementGetParameterSchemaRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementGetParameterSchemaRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementGetParameterSchemaResponse. */
    interface IStatementGetParameterSchemaResponse {

        /** StatementGetParameterSchemaResponse schema */
        schema?: (database_driver_v1.IArrowSchemaPtr|null);
    }

    /** Represents a StatementGetParameterSchemaResponse. */
    class StatementGetParameterSchemaResponse implements IStatementGetParameterSchemaResponse {

        /**
         * Constructs a new StatementGetParameterSchemaResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementGetParameterSchemaResponse);

        /** StatementGetParameterSchemaResponse schema. */
        public schema?: (database_driver_v1.IArrowSchemaPtr|null);

        /**
         * Creates a new StatementGetParameterSchemaResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementGetParameterSchemaResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementGetParameterSchemaResponse): database_driver_v1.StatementGetParameterSchemaResponse;

        /**
         * Encodes the specified StatementGetParameterSchemaResponse message. Does not implicitly {@link database_driver_v1.StatementGetParameterSchemaResponse.verify|verify} messages.
         * @param message StatementGetParameterSchemaResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementGetParameterSchemaResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementGetParameterSchemaResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementGetParameterSchemaResponse.verify|verify} messages.
         * @param message StatementGetParameterSchemaResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementGetParameterSchemaResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementGetParameterSchemaResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementGetParameterSchemaResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementGetParameterSchemaResponse;

        /**
         * Decodes a StatementGetParameterSchemaResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementGetParameterSchemaResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementGetParameterSchemaResponse;

        /**
         * Verifies a StatementGetParameterSchemaResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementGetParameterSchemaResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementGetParameterSchemaResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementGetParameterSchemaResponse;

        /**
         * Creates a plain object from a StatementGetParameterSchemaResponse message. Also converts values to other types if specified.
         * @param message StatementGetParameterSchemaResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementGetParameterSchemaResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementGetParameterSchemaResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementGetParameterSchemaResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementExecuteQueryRequest. */
    interface IStatementExecuteQueryRequest {

        /** StatementExecuteQueryRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementExecuteQueryRequest bindings */
        bindings?: (database_driver_v1.IQueryBindings|null);
    }

    /** Represents a StatementExecuteQueryRequest. */
    class StatementExecuteQueryRequest implements IStatementExecuteQueryRequest {

        /**
         * Constructs a new StatementExecuteQueryRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementExecuteQueryRequest);

        /** StatementExecuteQueryRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementExecuteQueryRequest bindings. */
        public bindings?: (database_driver_v1.IQueryBindings|null);

        /**
         * Creates a new StatementExecuteQueryRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementExecuteQueryRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementExecuteQueryRequest): database_driver_v1.StatementExecuteQueryRequest;

        /**
         * Encodes the specified StatementExecuteQueryRequest message. Does not implicitly {@link database_driver_v1.StatementExecuteQueryRequest.verify|verify} messages.
         * @param message StatementExecuteQueryRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementExecuteQueryRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementExecuteQueryRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementExecuteQueryRequest.verify|verify} messages.
         * @param message StatementExecuteQueryRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementExecuteQueryRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementExecuteQueryRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementExecuteQueryRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementExecuteQueryRequest;

        /**
         * Decodes a StatementExecuteQueryRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementExecuteQueryRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementExecuteQueryRequest;

        /**
         * Verifies a StatementExecuteQueryRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementExecuteQueryRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementExecuteQueryRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementExecuteQueryRequest;

        /**
         * Creates a plain object from a StatementExecuteQueryRequest message. Also converts values to other types if specified.
         * @param message StatementExecuteQueryRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementExecuteQueryRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementExecuteQueryRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementExecuteQueryRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementExecuteQueryResponse. */
    interface IStatementExecuteQueryResponse {

        /** StatementExecuteQueryResponse result */
        result?: (database_driver_v1.IExecuteResult|null);
    }

    /** Represents a StatementExecuteQueryResponse. */
    class StatementExecuteQueryResponse implements IStatementExecuteQueryResponse {

        /**
         * Constructs a new StatementExecuteQueryResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementExecuteQueryResponse);

        /** StatementExecuteQueryResponse result. */
        public result?: (database_driver_v1.IExecuteResult|null);

        /**
         * Creates a new StatementExecuteQueryResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementExecuteQueryResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementExecuteQueryResponse): database_driver_v1.StatementExecuteQueryResponse;

        /**
         * Encodes the specified StatementExecuteQueryResponse message. Does not implicitly {@link database_driver_v1.StatementExecuteQueryResponse.verify|verify} messages.
         * @param message StatementExecuteQueryResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementExecuteQueryResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementExecuteQueryResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementExecuteQueryResponse.verify|verify} messages.
         * @param message StatementExecuteQueryResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementExecuteQueryResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementExecuteQueryResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementExecuteQueryResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementExecuteQueryResponse;

        /**
         * Decodes a StatementExecuteQueryResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementExecuteQueryResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementExecuteQueryResponse;

        /**
         * Verifies a StatementExecuteQueryResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementExecuteQueryResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementExecuteQueryResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementExecuteQueryResponse;

        /**
         * Creates a plain object from a StatementExecuteQueryResponse message. Also converts values to other types if specified.
         * @param message StatementExecuteQueryResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementExecuteQueryResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementExecuteQueryResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementExecuteQueryResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementExecutePartitionsRequest. */
    interface IStatementExecutePartitionsRequest {

        /** StatementExecutePartitionsRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);
    }

    /** Represents a StatementExecutePartitionsRequest. */
    class StatementExecutePartitionsRequest implements IStatementExecutePartitionsRequest {

        /**
         * Constructs a new StatementExecutePartitionsRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementExecutePartitionsRequest);

        /** StatementExecutePartitionsRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /**
         * Creates a new StatementExecutePartitionsRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementExecutePartitionsRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementExecutePartitionsRequest): database_driver_v1.StatementExecutePartitionsRequest;

        /**
         * Encodes the specified StatementExecutePartitionsRequest message. Does not implicitly {@link database_driver_v1.StatementExecutePartitionsRequest.verify|verify} messages.
         * @param message StatementExecutePartitionsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementExecutePartitionsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementExecutePartitionsRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementExecutePartitionsRequest.verify|verify} messages.
         * @param message StatementExecutePartitionsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementExecutePartitionsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementExecutePartitionsRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementExecutePartitionsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementExecutePartitionsRequest;

        /**
         * Decodes a StatementExecutePartitionsRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementExecutePartitionsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementExecutePartitionsRequest;

        /**
         * Verifies a StatementExecutePartitionsRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementExecutePartitionsRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementExecutePartitionsRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementExecutePartitionsRequest;

        /**
         * Creates a plain object from a StatementExecutePartitionsRequest message. Also converts values to other types if specified.
         * @param message StatementExecutePartitionsRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementExecutePartitionsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementExecutePartitionsRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementExecutePartitionsRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementExecutePartitionsResponse. */
    interface IStatementExecutePartitionsResponse {

        /** StatementExecutePartitionsResponse result */
        result?: (database_driver_v1.IPartitionedResult|null);
    }

    /** Represents a StatementExecutePartitionsResponse. */
    class StatementExecutePartitionsResponse implements IStatementExecutePartitionsResponse {

        /**
         * Constructs a new StatementExecutePartitionsResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementExecutePartitionsResponse);

        /** StatementExecutePartitionsResponse result. */
        public result?: (database_driver_v1.IPartitionedResult|null);

        /**
         * Creates a new StatementExecutePartitionsResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementExecutePartitionsResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementExecutePartitionsResponse): database_driver_v1.StatementExecutePartitionsResponse;

        /**
         * Encodes the specified StatementExecutePartitionsResponse message. Does not implicitly {@link database_driver_v1.StatementExecutePartitionsResponse.verify|verify} messages.
         * @param message StatementExecutePartitionsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementExecutePartitionsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementExecutePartitionsResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementExecutePartitionsResponse.verify|verify} messages.
         * @param message StatementExecutePartitionsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementExecutePartitionsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementExecutePartitionsResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementExecutePartitionsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementExecutePartitionsResponse;

        /**
         * Decodes a StatementExecutePartitionsResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementExecutePartitionsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementExecutePartitionsResponse;

        /**
         * Verifies a StatementExecutePartitionsResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementExecutePartitionsResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementExecutePartitionsResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementExecutePartitionsResponse;

        /**
         * Creates a plain object from a StatementExecutePartitionsResponse message. Also converts values to other types if specified.
         * @param message StatementExecutePartitionsResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementExecutePartitionsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementExecutePartitionsResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementExecutePartitionsResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementReadPartitionRequest. */
    interface IStatementReadPartitionRequest {

        /** StatementReadPartitionRequest stmtHandle */
        stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementReadPartitionRequest partitionDescriptor */
        partitionDescriptor?: (Uint8Array|null);
    }

    /** Represents a StatementReadPartitionRequest. */
    class StatementReadPartitionRequest implements IStatementReadPartitionRequest {

        /**
         * Constructs a new StatementReadPartitionRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementReadPartitionRequest);

        /** StatementReadPartitionRequest stmtHandle. */
        public stmtHandle?: (database_driver_v1.IStatementHandle|null);

        /** StatementReadPartitionRequest partitionDescriptor. */
        public partitionDescriptor: Uint8Array;

        /**
         * Creates a new StatementReadPartitionRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementReadPartitionRequest instance
         */
        public static create(properties?: database_driver_v1.IStatementReadPartitionRequest): database_driver_v1.StatementReadPartitionRequest;

        /**
         * Encodes the specified StatementReadPartitionRequest message. Does not implicitly {@link database_driver_v1.StatementReadPartitionRequest.verify|verify} messages.
         * @param message StatementReadPartitionRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementReadPartitionRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementReadPartitionRequest message, length delimited. Does not implicitly {@link database_driver_v1.StatementReadPartitionRequest.verify|verify} messages.
         * @param message StatementReadPartitionRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementReadPartitionRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementReadPartitionRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementReadPartitionRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementReadPartitionRequest;

        /**
         * Decodes a StatementReadPartitionRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementReadPartitionRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementReadPartitionRequest;

        /**
         * Verifies a StatementReadPartitionRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementReadPartitionRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementReadPartitionRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementReadPartitionRequest;

        /**
         * Creates a plain object from a StatementReadPartitionRequest message. Also converts values to other types if specified.
         * @param message StatementReadPartitionRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementReadPartitionRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementReadPartitionRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementReadPartitionRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a StatementReadPartitionResponse. */
    interface IStatementReadPartitionResponse {

        /** StatementReadPartitionResponse partitionStream */
        partitionStream?: (number|Long|null);
    }

    /** Represents a StatementReadPartitionResponse. */
    class StatementReadPartitionResponse implements IStatementReadPartitionResponse {

        /**
         * Constructs a new StatementReadPartitionResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IStatementReadPartitionResponse);

        /** StatementReadPartitionResponse partitionStream. */
        public partitionStream: (number|Long);

        /**
         * Creates a new StatementReadPartitionResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns StatementReadPartitionResponse instance
         */
        public static create(properties?: database_driver_v1.IStatementReadPartitionResponse): database_driver_v1.StatementReadPartitionResponse;

        /**
         * Encodes the specified StatementReadPartitionResponse message. Does not implicitly {@link database_driver_v1.StatementReadPartitionResponse.verify|verify} messages.
         * @param message StatementReadPartitionResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IStatementReadPartitionResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified StatementReadPartitionResponse message, length delimited. Does not implicitly {@link database_driver_v1.StatementReadPartitionResponse.verify|verify} messages.
         * @param message StatementReadPartitionResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IStatementReadPartitionResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a StatementReadPartitionResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns StatementReadPartitionResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.StatementReadPartitionResponse;

        /**
         * Decodes a StatementReadPartitionResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns StatementReadPartitionResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.StatementReadPartitionResponse;

        /**
         * Verifies a StatementReadPartitionResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a StatementReadPartitionResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns StatementReadPartitionResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.StatementReadPartitionResponse;

        /**
         * Creates a plain object from a StatementReadPartitionResponse message. Also converts values to other types if specified.
         * @param message StatementReadPartitionResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.StatementReadPartitionResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this StatementReadPartitionResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for StatementReadPartitionResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConfigSetting. */
    interface IConfigSetting {

        /** ConfigSetting stringValue */
        stringValue?: (string|null);

        /** ConfigSetting intValue */
        intValue?: (number|Long|null);

        /** ConfigSetting doubleValue */
        doubleValue?: (number|null);

        /** ConfigSetting bytesValue */
        bytesValue?: (Uint8Array|null);

        /** ConfigSetting boolValue */
        boolValue?: (boolean|null);
    }

    /** Represents a ConfigSetting. */
    class ConfigSetting implements IConfigSetting {

        /**
         * Constructs a new ConfigSetting.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConfigSetting);

        /** ConfigSetting stringValue. */
        public stringValue?: (string|null);

        /** ConfigSetting intValue. */
        public intValue?: (number|Long|null);

        /** ConfigSetting doubleValue. */
        public doubleValue?: (number|null);

        /** ConfigSetting bytesValue. */
        public bytesValue?: (Uint8Array|null);

        /** ConfigSetting boolValue. */
        public boolValue?: (boolean|null);

        /** ConfigSetting value. */
        public value?: ("stringValue"|"intValue"|"doubleValue"|"bytesValue"|"boolValue");

        /**
         * Creates a new ConfigSetting instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConfigSetting instance
         */
        public static create(properties?: database_driver_v1.IConfigSetting): database_driver_v1.ConfigSetting;

        /**
         * Encodes the specified ConfigSetting message. Does not implicitly {@link database_driver_v1.ConfigSetting.verify|verify} messages.
         * @param message ConfigSetting message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConfigSetting, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConfigSetting message, length delimited. Does not implicitly {@link database_driver_v1.ConfigSetting.verify|verify} messages.
         * @param message ConfigSetting message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConfigSetting, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConfigSetting message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConfigSetting
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConfigSetting;

        /**
         * Decodes a ConfigSetting message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConfigSetting
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConfigSetting;

        /**
         * Verifies a ConfigSetting message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConfigSetting message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConfigSetting
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConfigSetting;

        /**
         * Creates a plain object from a ConfigSetting message. Also converts values to other types if specified.
         * @param message ConfigSetting
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConfigSetting, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConfigSetting to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConfigSetting
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConfigSection. */
    interface IConfigSection {

        /** ConfigSection settings */
        settings?: ({ [k: string]: database_driver_v1.IConfigSetting }|null);
    }

    /** Represents a ConfigSection. */
    class ConfigSection implements IConfigSection {

        /**
         * Constructs a new ConfigSection.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConfigSection);

        /** ConfigSection settings. */
        public settings: { [k: string]: database_driver_v1.IConfigSetting };

        /**
         * Creates a new ConfigSection instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConfigSection instance
         */
        public static create(properties?: database_driver_v1.IConfigSection): database_driver_v1.ConfigSection;

        /**
         * Encodes the specified ConfigSection message. Does not implicitly {@link database_driver_v1.ConfigSection.verify|verify} messages.
         * @param message ConfigSection message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConfigSection, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConfigSection message, length delimited. Does not implicitly {@link database_driver_v1.ConfigSection.verify|verify} messages.
         * @param message ConfigSection message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConfigSection, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConfigSection message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConfigSection
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConfigSection;

        /**
         * Decodes a ConfigSection message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConfigSection
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConfigSection;

        /**
         * Verifies a ConfigSection message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConfigSection message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConfigSection
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConfigSection;

        /**
         * Creates a plain object from a ConfigSection message. Also converts values to other types if specified.
         * @param message ConfigSection
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConfigSection, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConfigSection to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConfigSection
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConfigLoadAllSectionsRequest. */
    interface IConfigLoadAllSectionsRequest {

        /** ConfigLoadAllSectionsRequest configFile */
        configFile?: (string|null);

        /** ConfigLoadAllSectionsRequest connectionsFile */
        connectionsFile?: (string|null);
    }

    /** Represents a ConfigLoadAllSectionsRequest. */
    class ConfigLoadAllSectionsRequest implements IConfigLoadAllSectionsRequest {

        /**
         * Constructs a new ConfigLoadAllSectionsRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConfigLoadAllSectionsRequest);

        /** ConfigLoadAllSectionsRequest configFile. */
        public configFile?: (string|null);

        /** ConfigLoadAllSectionsRequest connectionsFile. */
        public connectionsFile?: (string|null);

        /**
         * Creates a new ConfigLoadAllSectionsRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConfigLoadAllSectionsRequest instance
         */
        public static create(properties?: database_driver_v1.IConfigLoadAllSectionsRequest): database_driver_v1.ConfigLoadAllSectionsRequest;

        /**
         * Encodes the specified ConfigLoadAllSectionsRequest message. Does not implicitly {@link database_driver_v1.ConfigLoadAllSectionsRequest.verify|verify} messages.
         * @param message ConfigLoadAllSectionsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConfigLoadAllSectionsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConfigLoadAllSectionsRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConfigLoadAllSectionsRequest.verify|verify} messages.
         * @param message ConfigLoadAllSectionsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConfigLoadAllSectionsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConfigLoadAllSectionsRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConfigLoadAllSectionsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConfigLoadAllSectionsRequest;

        /**
         * Decodes a ConfigLoadAllSectionsRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConfigLoadAllSectionsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConfigLoadAllSectionsRequest;

        /**
         * Verifies a ConfigLoadAllSectionsRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConfigLoadAllSectionsRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConfigLoadAllSectionsRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConfigLoadAllSectionsRequest;

        /**
         * Creates a plain object from a ConfigLoadAllSectionsRequest message. Also converts values to other types if specified.
         * @param message ConfigLoadAllSectionsRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConfigLoadAllSectionsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConfigLoadAllSectionsRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConfigLoadAllSectionsRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConfigLoadAllSectionsResponse. */
    interface IConfigLoadAllSectionsResponse {

        /** ConfigLoadAllSectionsResponse configJson */
        configJson?: (string|null);
    }

    /** Represents a ConfigLoadAllSectionsResponse. */
    class ConfigLoadAllSectionsResponse implements IConfigLoadAllSectionsResponse {

        /**
         * Constructs a new ConfigLoadAllSectionsResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConfigLoadAllSectionsResponse);

        /** ConfigLoadAllSectionsResponse configJson. */
        public configJson: string;

        /**
         * Creates a new ConfigLoadAllSectionsResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConfigLoadAllSectionsResponse instance
         */
        public static create(properties?: database_driver_v1.IConfigLoadAllSectionsResponse): database_driver_v1.ConfigLoadAllSectionsResponse;

        /**
         * Encodes the specified ConfigLoadAllSectionsResponse message. Does not implicitly {@link database_driver_v1.ConfigLoadAllSectionsResponse.verify|verify} messages.
         * @param message ConfigLoadAllSectionsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConfigLoadAllSectionsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConfigLoadAllSectionsResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConfigLoadAllSectionsResponse.verify|verify} messages.
         * @param message ConfigLoadAllSectionsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConfigLoadAllSectionsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConfigLoadAllSectionsResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConfigLoadAllSectionsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConfigLoadAllSectionsResponse;

        /**
         * Decodes a ConfigLoadAllSectionsResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConfigLoadAllSectionsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConfigLoadAllSectionsResponse;

        /**
         * Verifies a ConfigLoadAllSectionsResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConfigLoadAllSectionsResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConfigLoadAllSectionsResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConfigLoadAllSectionsResponse;

        /**
         * Creates a plain object from a ConfigLoadAllSectionsResponse message. Also converts values to other types if specified.
         * @param message ConfigLoadAllSectionsResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConfigLoadAllSectionsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConfigLoadAllSectionsResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConfigLoadAllSectionsResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConfigGetPathsRequest. */
    interface IConfigGetPathsRequest {
    }

    /** Represents a ConfigGetPathsRequest. */
    class ConfigGetPathsRequest implements IConfigGetPathsRequest {

        /**
         * Constructs a new ConfigGetPathsRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConfigGetPathsRequest);

        /**
         * Creates a new ConfigGetPathsRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConfigGetPathsRequest instance
         */
        public static create(properties?: database_driver_v1.IConfigGetPathsRequest): database_driver_v1.ConfigGetPathsRequest;

        /**
         * Encodes the specified ConfigGetPathsRequest message. Does not implicitly {@link database_driver_v1.ConfigGetPathsRequest.verify|verify} messages.
         * @param message ConfigGetPathsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConfigGetPathsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConfigGetPathsRequest message, length delimited. Does not implicitly {@link database_driver_v1.ConfigGetPathsRequest.verify|verify} messages.
         * @param message ConfigGetPathsRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConfigGetPathsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConfigGetPathsRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConfigGetPathsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConfigGetPathsRequest;

        /**
         * Decodes a ConfigGetPathsRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConfigGetPathsRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConfigGetPathsRequest;

        /**
         * Verifies a ConfigGetPathsRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConfigGetPathsRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConfigGetPathsRequest
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConfigGetPathsRequest;

        /**
         * Creates a plain object from a ConfigGetPathsRequest message. Also converts values to other types if specified.
         * @param message ConfigGetPathsRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConfigGetPathsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConfigGetPathsRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConfigGetPathsRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ConfigGetPathsResponse. */
    interface IConfigGetPathsResponse {

        /** ConfigGetPathsResponse configFile */
        configFile?: (string|null);

        /** ConfigGetPathsResponse connectionsFile */
        connectionsFile?: (string|null);
    }

    /** Represents a ConfigGetPathsResponse. */
    class ConfigGetPathsResponse implements IConfigGetPathsResponse {

        /**
         * Constructs a new ConfigGetPathsResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: database_driver_v1.IConfigGetPathsResponse);

        /** ConfigGetPathsResponse configFile. */
        public configFile: string;

        /** ConfigGetPathsResponse connectionsFile. */
        public connectionsFile: string;

        /**
         * Creates a new ConfigGetPathsResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ConfigGetPathsResponse instance
         */
        public static create(properties?: database_driver_v1.IConfigGetPathsResponse): database_driver_v1.ConfigGetPathsResponse;

        /**
         * Encodes the specified ConfigGetPathsResponse message. Does not implicitly {@link database_driver_v1.ConfigGetPathsResponse.verify|verify} messages.
         * @param message ConfigGetPathsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: database_driver_v1.IConfigGetPathsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ConfigGetPathsResponse message, length delimited. Does not implicitly {@link database_driver_v1.ConfigGetPathsResponse.verify|verify} messages.
         * @param message ConfigGetPathsResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: database_driver_v1.IConfigGetPathsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ConfigGetPathsResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ConfigGetPathsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): database_driver_v1.ConfigGetPathsResponse;

        /**
         * Decodes a ConfigGetPathsResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ConfigGetPathsResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): database_driver_v1.ConfigGetPathsResponse;

        /**
         * Verifies a ConfigGetPathsResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ConfigGetPathsResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ConfigGetPathsResponse
         */
        public static fromObject(object: { [k: string]: any }): database_driver_v1.ConfigGetPathsResponse;

        /**
         * Creates a plain object from a ConfigGetPathsResponse message. Also converts values to other types if specified.
         * @param message ConfigGetPathsResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: database_driver_v1.ConfigGetPathsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ConfigGetPathsResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ConfigGetPathsResponse
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Represents a DatabaseDriver */
    class DatabaseDriver extends $protobuf.rpc.Service {

        /**
         * Constructs a new DatabaseDriver service.
         * @param rpcImpl RPC implementation
         * @param [requestDelimited=false] Whether requests are length-delimited
         * @param [responseDelimited=false] Whether responses are length-delimited
         */
        constructor(rpcImpl: $protobuf.RPCImpl, requestDelimited?: boolean, responseDelimited?: boolean);

        /**
         * Creates new DatabaseDriver service using the specified rpc implementation.
         * @param rpcImpl RPC implementation
         * @param [requestDelimited=false] Whether requests are length-delimited
         * @param [responseDelimited=false] Whether responses are length-delimited
         * @returns RPC service. Useful where requests and/or responses are streamed.
         */
        public static create(rpcImpl: $protobuf.RPCImpl, requestDelimited?: boolean, responseDelimited?: boolean): DatabaseDriver;

        /**
         * Calls DatabaseNew.
         * @param request DatabaseNewRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseNewResponse
         */
        public databaseNew(request: database_driver_v1.IDatabaseNewRequest, callback: database_driver_v1.DatabaseDriver.DatabaseNewCallback): void;

        /**
         * Calls DatabaseNew.
         * @param request DatabaseNewRequest message or plain object
         * @returns Promise
         */
        public databaseNew(request: database_driver_v1.IDatabaseNewRequest): Promise<database_driver_v1.DatabaseNewResponse>;

        /**
         * Calls DatabaseSetOptionString.
         * @param request DatabaseSetOptionStringRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseSetOptionStringResponse
         */
        public databaseSetOptionString(request: database_driver_v1.IDatabaseSetOptionStringRequest, callback: database_driver_v1.DatabaseDriver.DatabaseSetOptionStringCallback): void;

        /**
         * Calls DatabaseSetOptionString.
         * @param request DatabaseSetOptionStringRequest message or plain object
         * @returns Promise
         */
        public databaseSetOptionString(request: database_driver_v1.IDatabaseSetOptionStringRequest): Promise<database_driver_v1.DatabaseSetOptionStringResponse>;

        /**
         * Calls DatabaseSetOptionBytes.
         * @param request DatabaseSetOptionBytesRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseSetOptionBytesResponse
         */
        public databaseSetOptionBytes(request: database_driver_v1.IDatabaseSetOptionBytesRequest, callback: database_driver_v1.DatabaseDriver.DatabaseSetOptionBytesCallback): void;

        /**
         * Calls DatabaseSetOptionBytes.
         * @param request DatabaseSetOptionBytesRequest message or plain object
         * @returns Promise
         */
        public databaseSetOptionBytes(request: database_driver_v1.IDatabaseSetOptionBytesRequest): Promise<database_driver_v1.DatabaseSetOptionBytesResponse>;

        /**
         * Calls DatabaseSetOptionInt.
         * @param request DatabaseSetOptionIntRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseSetOptionIntResponse
         */
        public databaseSetOptionInt(request: database_driver_v1.IDatabaseSetOptionIntRequest, callback: database_driver_v1.DatabaseDriver.DatabaseSetOptionIntCallback): void;

        /**
         * Calls DatabaseSetOptionInt.
         * @param request DatabaseSetOptionIntRequest message or plain object
         * @returns Promise
         */
        public databaseSetOptionInt(request: database_driver_v1.IDatabaseSetOptionIntRequest): Promise<database_driver_v1.DatabaseSetOptionIntResponse>;

        /**
         * Calls DatabaseSetOptionDouble.
         * @param request DatabaseSetOptionDoubleRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseSetOptionDoubleResponse
         */
        public databaseSetOptionDouble(request: database_driver_v1.IDatabaseSetOptionDoubleRequest, callback: database_driver_v1.DatabaseDriver.DatabaseSetOptionDoubleCallback): void;

        /**
         * Calls DatabaseSetOptionDouble.
         * @param request DatabaseSetOptionDoubleRequest message or plain object
         * @returns Promise
         */
        public databaseSetOptionDouble(request: database_driver_v1.IDatabaseSetOptionDoubleRequest): Promise<database_driver_v1.DatabaseSetOptionDoubleResponse>;

        /**
         * Calls DatabaseInit.
         * @param request DatabaseInitRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseInitResponse
         */
        public databaseInit(request: database_driver_v1.IDatabaseInitRequest, callback: database_driver_v1.DatabaseDriver.DatabaseInitCallback): void;

        /**
         * Calls DatabaseInit.
         * @param request DatabaseInitRequest message or plain object
         * @returns Promise
         */
        public databaseInit(request: database_driver_v1.IDatabaseInitRequest): Promise<database_driver_v1.DatabaseInitResponse>;

        /**
         * Calls DatabaseRelease.
         * @param request DatabaseReleaseRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and DatabaseReleaseResponse
         */
        public databaseRelease(request: database_driver_v1.IDatabaseReleaseRequest, callback: database_driver_v1.DatabaseDriver.DatabaseReleaseCallback): void;

        /**
         * Calls DatabaseRelease.
         * @param request DatabaseReleaseRequest message or plain object
         * @returns Promise
         */
        public databaseRelease(request: database_driver_v1.IDatabaseReleaseRequest): Promise<database_driver_v1.DatabaseReleaseResponse>;

        /**
         * Calls ConnectionNew.
         * @param request ConnectionNewRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionNewResponse
         */
        public connectionNew(request: database_driver_v1.IConnectionNewRequest, callback: database_driver_v1.DatabaseDriver.ConnectionNewCallback): void;

        /**
         * Calls ConnectionNew.
         * @param request ConnectionNewRequest message or plain object
         * @returns Promise
         */
        public connectionNew(request: database_driver_v1.IConnectionNewRequest): Promise<database_driver_v1.ConnectionNewResponse>;

        /**
         * Calls ConnectionSetOptionString.
         * @param request ConnectionSetOptionStringRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionSetOptionStringResponse
         */
        public connectionSetOptionString(request: database_driver_v1.IConnectionSetOptionStringRequest, callback: database_driver_v1.DatabaseDriver.ConnectionSetOptionStringCallback): void;

        /**
         * Calls ConnectionSetOptionString.
         * @param request ConnectionSetOptionStringRequest message or plain object
         * @returns Promise
         */
        public connectionSetOptionString(request: database_driver_v1.IConnectionSetOptionStringRequest): Promise<database_driver_v1.ConnectionSetOptionStringResponse>;

        /**
         * Calls ConnectionSetOptionBytes.
         * @param request ConnectionSetOptionBytesRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionSetOptionBytesResponse
         */
        public connectionSetOptionBytes(request: database_driver_v1.IConnectionSetOptionBytesRequest, callback: database_driver_v1.DatabaseDriver.ConnectionSetOptionBytesCallback): void;

        /**
         * Calls ConnectionSetOptionBytes.
         * @param request ConnectionSetOptionBytesRequest message or plain object
         * @returns Promise
         */
        public connectionSetOptionBytes(request: database_driver_v1.IConnectionSetOptionBytesRequest): Promise<database_driver_v1.ConnectionSetOptionBytesResponse>;

        /**
         * Calls ConnectionSetOptionInt.
         * @param request ConnectionSetOptionIntRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionSetOptionIntResponse
         */
        public connectionSetOptionInt(request: database_driver_v1.IConnectionSetOptionIntRequest, callback: database_driver_v1.DatabaseDriver.ConnectionSetOptionIntCallback): void;

        /**
         * Calls ConnectionSetOptionInt.
         * @param request ConnectionSetOptionIntRequest message or plain object
         * @returns Promise
         */
        public connectionSetOptionInt(request: database_driver_v1.IConnectionSetOptionIntRequest): Promise<database_driver_v1.ConnectionSetOptionIntResponse>;

        /**
         * Calls ConnectionSetOptionDouble.
         * @param request ConnectionSetOptionDoubleRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionSetOptionDoubleResponse
         */
        public connectionSetOptionDouble(request: database_driver_v1.IConnectionSetOptionDoubleRequest, callback: database_driver_v1.DatabaseDriver.ConnectionSetOptionDoubleCallback): void;

        /**
         * Calls ConnectionSetOptionDouble.
         * @param request ConnectionSetOptionDoubleRequest message or plain object
         * @returns Promise
         */
        public connectionSetOptionDouble(request: database_driver_v1.IConnectionSetOptionDoubleRequest): Promise<database_driver_v1.ConnectionSetOptionDoubleResponse>;

        /**
         * Calls ConnectionInit.
         * @param request ConnectionInitRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionInitResponse
         */
        public connectionInit(request: database_driver_v1.IConnectionInitRequest, callback: database_driver_v1.DatabaseDriver.ConnectionInitCallback): void;

        /**
         * Calls ConnectionInit.
         * @param request ConnectionInitRequest message or plain object
         * @returns Promise
         */
        public connectionInit(request: database_driver_v1.IConnectionInitRequest): Promise<database_driver_v1.ConnectionInitResponse>;

        /**
         * Calls ConnectionRelease.
         * @param request ConnectionReleaseRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionReleaseResponse
         */
        public connectionRelease(request: database_driver_v1.IConnectionReleaseRequest, callback: database_driver_v1.DatabaseDriver.ConnectionReleaseCallback): void;

        /**
         * Calls ConnectionRelease.
         * @param request ConnectionReleaseRequest message or plain object
         * @returns Promise
         */
        public connectionRelease(request: database_driver_v1.IConnectionReleaseRequest): Promise<database_driver_v1.ConnectionReleaseResponse>;

        /**
         * Calls ConnectionGetInfo.
         * @param request ConnectionGetInfoRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionGetInfoResponse
         */
        public connectionGetInfo(request: database_driver_v1.IConnectionGetInfoRequest, callback: database_driver_v1.DatabaseDriver.ConnectionGetInfoCallback): void;

        /**
         * Calls ConnectionGetInfo.
         * @param request ConnectionGetInfoRequest message or plain object
         * @returns Promise
         */
        public connectionGetInfo(request: database_driver_v1.IConnectionGetInfoRequest): Promise<database_driver_v1.ConnectionGetInfoResponse>;

        /**
         * Calls ConnectionGetObjects.
         * @param request ConnectionGetObjectsRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionGetObjectsResponse
         */
        public connectionGetObjects(request: database_driver_v1.IConnectionGetObjectsRequest, callback: database_driver_v1.DatabaseDriver.ConnectionGetObjectsCallback): void;

        /**
         * Calls ConnectionGetObjects.
         * @param request ConnectionGetObjectsRequest message or plain object
         * @returns Promise
         */
        public connectionGetObjects(request: database_driver_v1.IConnectionGetObjectsRequest): Promise<database_driver_v1.ConnectionGetObjectsResponse>;

        /**
         * Calls ConnectionGetTableSchema.
         * @param request ConnectionGetTableSchemaRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionGetTableSchemaResponse
         */
        public connectionGetTableSchema(request: database_driver_v1.IConnectionGetTableSchemaRequest, callback: database_driver_v1.DatabaseDriver.ConnectionGetTableSchemaCallback): void;

        /**
         * Calls ConnectionGetTableSchema.
         * @param request ConnectionGetTableSchemaRequest message or plain object
         * @returns Promise
         */
        public connectionGetTableSchema(request: database_driver_v1.IConnectionGetTableSchemaRequest): Promise<database_driver_v1.ConnectionGetTableSchemaResponse>;

        /**
         * Calls ConnectionGetTableTypes.
         * @param request ConnectionGetTableTypesRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionGetTableTypesResponse
         */
        public connectionGetTableTypes(request: database_driver_v1.IConnectionGetTableTypesRequest, callback: database_driver_v1.DatabaseDriver.ConnectionGetTableTypesCallback): void;

        /**
         * Calls ConnectionGetTableTypes.
         * @param request ConnectionGetTableTypesRequest message or plain object
         * @returns Promise
         */
        public connectionGetTableTypes(request: database_driver_v1.IConnectionGetTableTypesRequest): Promise<database_driver_v1.ConnectionGetTableTypesResponse>;

        /**
         * Calls ConnectionCommit.
         * @param request ConnectionCommitRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionCommitResponse
         */
        public connectionCommit(request: database_driver_v1.IConnectionCommitRequest, callback: database_driver_v1.DatabaseDriver.ConnectionCommitCallback): void;

        /**
         * Calls ConnectionCommit.
         * @param request ConnectionCommitRequest message or plain object
         * @returns Promise
         */
        public connectionCommit(request: database_driver_v1.IConnectionCommitRequest): Promise<database_driver_v1.ConnectionCommitResponse>;

        /**
         * Calls ConnectionRollback.
         * @param request ConnectionRollbackRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionRollbackResponse
         */
        public connectionRollback(request: database_driver_v1.IConnectionRollbackRequest, callback: database_driver_v1.DatabaseDriver.ConnectionRollbackCallback): void;

        /**
         * Calls ConnectionRollback.
         * @param request ConnectionRollbackRequest message or plain object
         * @returns Promise
         */
        public connectionRollback(request: database_driver_v1.IConnectionRollbackRequest): Promise<database_driver_v1.ConnectionRollbackResponse>;

        /**
         * Calls ConnectionSetSessionParameters.
         * @param request ConnectionSetSessionParametersRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionSetSessionParametersResponse
         */
        public connectionSetSessionParameters(request: database_driver_v1.IConnectionSetSessionParametersRequest, callback: database_driver_v1.DatabaseDriver.ConnectionSetSessionParametersCallback): void;

        /**
         * Calls ConnectionSetSessionParameters.
         * @param request ConnectionSetSessionParametersRequest message or plain object
         * @returns Promise
         */
        public connectionSetSessionParameters(request: database_driver_v1.IConnectionSetSessionParametersRequest): Promise<database_driver_v1.ConnectionSetSessionParametersResponse>;

        /**
         * Calls ConnectionGetParameter.
         * @param request ConnectionGetParameterRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConnectionGetParameterResponse
         */
        public connectionGetParameter(request: database_driver_v1.IConnectionGetParameterRequest, callback: database_driver_v1.DatabaseDriver.ConnectionGetParameterCallback): void;

        /**
         * Calls ConnectionGetParameter.
         * @param request ConnectionGetParameterRequest message or plain object
         * @returns Promise
         */
        public connectionGetParameter(request: database_driver_v1.IConnectionGetParameterRequest): Promise<database_driver_v1.ConnectionGetParameterResponse>;

        /**
         * Calls StatementNew.
         * @param request StatementNewRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementNewResponse
         */
        public statementNew(request: database_driver_v1.IStatementNewRequest, callback: database_driver_v1.DatabaseDriver.StatementNewCallback): void;

        /**
         * Calls StatementNew.
         * @param request StatementNewRequest message or plain object
         * @returns Promise
         */
        public statementNew(request: database_driver_v1.IStatementNewRequest): Promise<database_driver_v1.StatementNewResponse>;

        /**
         * Calls StatementRelease.
         * @param request StatementReleaseRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementReleaseResponse
         */
        public statementRelease(request: database_driver_v1.IStatementReleaseRequest, callback: database_driver_v1.DatabaseDriver.StatementReleaseCallback): void;

        /**
         * Calls StatementRelease.
         * @param request StatementReleaseRequest message or plain object
         * @returns Promise
         */
        public statementRelease(request: database_driver_v1.IStatementReleaseRequest): Promise<database_driver_v1.StatementReleaseResponse>;

        /**
         * Calls StatementSetSqlQuery.
         * @param request StatementSetSqlQueryRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementSetSqlQueryResponse
         */
        public statementSetSqlQuery(request: database_driver_v1.IStatementSetSqlQueryRequest, callback: database_driver_v1.DatabaseDriver.StatementSetSqlQueryCallback): void;

        /**
         * Calls StatementSetSqlQuery.
         * @param request StatementSetSqlQueryRequest message or plain object
         * @returns Promise
         */
        public statementSetSqlQuery(request: database_driver_v1.IStatementSetSqlQueryRequest): Promise<database_driver_v1.StatementSetSqlQueryResponse>;

        /**
         * Calls StatementSetSubstraitPlan.
         * @param request StatementSetSubstraitPlanRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementSetSubstraitPlanResponse
         */
        public statementSetSubstraitPlan(request: database_driver_v1.IStatementSetSubstraitPlanRequest, callback: database_driver_v1.DatabaseDriver.StatementSetSubstraitPlanCallback): void;

        /**
         * Calls StatementSetSubstraitPlan.
         * @param request StatementSetSubstraitPlanRequest message or plain object
         * @returns Promise
         */
        public statementSetSubstraitPlan(request: database_driver_v1.IStatementSetSubstraitPlanRequest): Promise<database_driver_v1.StatementSetSubstraitPlanResponse>;

        /**
         * Calls StatementPrepare.
         * @param request StatementPrepareRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementPrepareResponse
         */
        public statementPrepare(request: database_driver_v1.IStatementPrepareRequest, callback: database_driver_v1.DatabaseDriver.StatementPrepareCallback): void;

        /**
         * Calls StatementPrepare.
         * @param request StatementPrepareRequest message or plain object
         * @returns Promise
         */
        public statementPrepare(request: database_driver_v1.IStatementPrepareRequest): Promise<database_driver_v1.StatementPrepareResponse>;

        /**
         * Calls StatementSetOptionString.
         * @param request StatementSetOptionStringRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementSetOptionStringResponse
         */
        public statementSetOptionString(request: database_driver_v1.IStatementSetOptionStringRequest, callback: database_driver_v1.DatabaseDriver.StatementSetOptionStringCallback): void;

        /**
         * Calls StatementSetOptionString.
         * @param request StatementSetOptionStringRequest message or plain object
         * @returns Promise
         */
        public statementSetOptionString(request: database_driver_v1.IStatementSetOptionStringRequest): Promise<database_driver_v1.StatementSetOptionStringResponse>;

        /**
         * Calls StatementSetOptionBytes.
         * @param request StatementSetOptionBytesRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementSetOptionBytesResponse
         */
        public statementSetOptionBytes(request: database_driver_v1.IStatementSetOptionBytesRequest, callback: database_driver_v1.DatabaseDriver.StatementSetOptionBytesCallback): void;

        /**
         * Calls StatementSetOptionBytes.
         * @param request StatementSetOptionBytesRequest message or plain object
         * @returns Promise
         */
        public statementSetOptionBytes(request: database_driver_v1.IStatementSetOptionBytesRequest): Promise<database_driver_v1.StatementSetOptionBytesResponse>;

        /**
         * Calls StatementSetOptionInt.
         * @param request StatementSetOptionIntRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementSetOptionIntResponse
         */
        public statementSetOptionInt(request: database_driver_v1.IStatementSetOptionIntRequest, callback: database_driver_v1.DatabaseDriver.StatementSetOptionIntCallback): void;

        /**
         * Calls StatementSetOptionInt.
         * @param request StatementSetOptionIntRequest message or plain object
         * @returns Promise
         */
        public statementSetOptionInt(request: database_driver_v1.IStatementSetOptionIntRequest): Promise<database_driver_v1.StatementSetOptionIntResponse>;

        /**
         * Calls StatementSetOptionDouble.
         * @param request StatementSetOptionDoubleRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementSetOptionDoubleResponse
         */
        public statementSetOptionDouble(request: database_driver_v1.IStatementSetOptionDoubleRequest, callback: database_driver_v1.DatabaseDriver.StatementSetOptionDoubleCallback): void;

        /**
         * Calls StatementSetOptionDouble.
         * @param request StatementSetOptionDoubleRequest message or plain object
         * @returns Promise
         */
        public statementSetOptionDouble(request: database_driver_v1.IStatementSetOptionDoubleRequest): Promise<database_driver_v1.StatementSetOptionDoubleResponse>;

        /**
         * Calls StatementGetParameterSchema.
         * @param request StatementGetParameterSchemaRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementGetParameterSchemaResponse
         */
        public statementGetParameterSchema(request: database_driver_v1.IStatementGetParameterSchemaRequest, callback: database_driver_v1.DatabaseDriver.StatementGetParameterSchemaCallback): void;

        /**
         * Calls StatementGetParameterSchema.
         * @param request StatementGetParameterSchemaRequest message or plain object
         * @returns Promise
         */
        public statementGetParameterSchema(request: database_driver_v1.IStatementGetParameterSchemaRequest): Promise<database_driver_v1.StatementGetParameterSchemaResponse>;

        /**
         * Calls StatementExecuteQuery.
         * @param request StatementExecuteQueryRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementExecuteQueryResponse
         */
        public statementExecuteQuery(request: database_driver_v1.IStatementExecuteQueryRequest, callback: database_driver_v1.DatabaseDriver.StatementExecuteQueryCallback): void;

        /**
         * Calls StatementExecuteQuery.
         * @param request StatementExecuteQueryRequest message or plain object
         * @returns Promise
         */
        public statementExecuteQuery(request: database_driver_v1.IStatementExecuteQueryRequest): Promise<database_driver_v1.StatementExecuteQueryResponse>;

        /**
         * Calls StatementExecutePartitions.
         * @param request StatementExecutePartitionsRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementExecutePartitionsResponse
         */
        public statementExecutePartitions(request: database_driver_v1.IStatementExecutePartitionsRequest, callback: database_driver_v1.DatabaseDriver.StatementExecutePartitionsCallback): void;

        /**
         * Calls StatementExecutePartitions.
         * @param request StatementExecutePartitionsRequest message or plain object
         * @returns Promise
         */
        public statementExecutePartitions(request: database_driver_v1.IStatementExecutePartitionsRequest): Promise<database_driver_v1.StatementExecutePartitionsResponse>;

        /**
         * Calls StatementReadPartition.
         * @param request StatementReadPartitionRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and StatementReadPartitionResponse
         */
        public statementReadPartition(request: database_driver_v1.IStatementReadPartitionRequest, callback: database_driver_v1.DatabaseDriver.StatementReadPartitionCallback): void;

        /**
         * Calls StatementReadPartition.
         * @param request StatementReadPartitionRequest message or plain object
         * @returns Promise
         */
        public statementReadPartition(request: database_driver_v1.IStatementReadPartitionRequest): Promise<database_driver_v1.StatementReadPartitionResponse>;

        /**
         * Calls ConfigLoadAllSections.
         * @param request ConfigLoadAllSectionsRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConfigLoadAllSectionsResponse
         */
        public configLoadAllSections(request: database_driver_v1.IConfigLoadAllSectionsRequest, callback: database_driver_v1.DatabaseDriver.ConfigLoadAllSectionsCallback): void;

        /**
         * Calls ConfigLoadAllSections.
         * @param request ConfigLoadAllSectionsRequest message or plain object
         * @returns Promise
         */
        public configLoadAllSections(request: database_driver_v1.IConfigLoadAllSectionsRequest): Promise<database_driver_v1.ConfigLoadAllSectionsResponse>;

        /**
         * Calls ConfigGetPaths.
         * @param request ConfigGetPathsRequest message or plain object
         * @param callback Node-style callback called with the error, if any, and ConfigGetPathsResponse
         */
        public configGetPaths(request: database_driver_v1.IConfigGetPathsRequest, callback: database_driver_v1.DatabaseDriver.ConfigGetPathsCallback): void;

        /**
         * Calls ConfigGetPaths.
         * @param request ConfigGetPathsRequest message or plain object
         * @returns Promise
         */
        public configGetPaths(request: database_driver_v1.IConfigGetPathsRequest): Promise<database_driver_v1.ConfigGetPathsResponse>;
    }

    namespace DatabaseDriver {

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseNew}.
         * @param error Error, if any
         * @param [response] DatabaseNewResponse
         */
        type DatabaseNewCallback = (error: (Error|null), response?: database_driver_v1.DatabaseNewResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseSetOptionString}.
         * @param error Error, if any
         * @param [response] DatabaseSetOptionStringResponse
         */
        type DatabaseSetOptionStringCallback = (error: (Error|null), response?: database_driver_v1.DatabaseSetOptionStringResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseSetOptionBytes}.
         * @param error Error, if any
         * @param [response] DatabaseSetOptionBytesResponse
         */
        type DatabaseSetOptionBytesCallback = (error: (Error|null), response?: database_driver_v1.DatabaseSetOptionBytesResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseSetOptionInt}.
         * @param error Error, if any
         * @param [response] DatabaseSetOptionIntResponse
         */
        type DatabaseSetOptionIntCallback = (error: (Error|null), response?: database_driver_v1.DatabaseSetOptionIntResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseSetOptionDouble}.
         * @param error Error, if any
         * @param [response] DatabaseSetOptionDoubleResponse
         */
        type DatabaseSetOptionDoubleCallback = (error: (Error|null), response?: database_driver_v1.DatabaseSetOptionDoubleResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseInit}.
         * @param error Error, if any
         * @param [response] DatabaseInitResponse
         */
        type DatabaseInitCallback = (error: (Error|null), response?: database_driver_v1.DatabaseInitResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#databaseRelease}.
         * @param error Error, if any
         * @param [response] DatabaseReleaseResponse
         */
        type DatabaseReleaseCallback = (error: (Error|null), response?: database_driver_v1.DatabaseReleaseResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionNew}.
         * @param error Error, if any
         * @param [response] ConnectionNewResponse
         */
        type ConnectionNewCallback = (error: (Error|null), response?: database_driver_v1.ConnectionNewResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionSetOptionString}.
         * @param error Error, if any
         * @param [response] ConnectionSetOptionStringResponse
         */
        type ConnectionSetOptionStringCallback = (error: (Error|null), response?: database_driver_v1.ConnectionSetOptionStringResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionSetOptionBytes}.
         * @param error Error, if any
         * @param [response] ConnectionSetOptionBytesResponse
         */
        type ConnectionSetOptionBytesCallback = (error: (Error|null), response?: database_driver_v1.ConnectionSetOptionBytesResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionSetOptionInt}.
         * @param error Error, if any
         * @param [response] ConnectionSetOptionIntResponse
         */
        type ConnectionSetOptionIntCallback = (error: (Error|null), response?: database_driver_v1.ConnectionSetOptionIntResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionSetOptionDouble}.
         * @param error Error, if any
         * @param [response] ConnectionSetOptionDoubleResponse
         */
        type ConnectionSetOptionDoubleCallback = (error: (Error|null), response?: database_driver_v1.ConnectionSetOptionDoubleResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionInit}.
         * @param error Error, if any
         * @param [response] ConnectionInitResponse
         */
        type ConnectionInitCallback = (error: (Error|null), response?: database_driver_v1.ConnectionInitResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionRelease}.
         * @param error Error, if any
         * @param [response] ConnectionReleaseResponse
         */
        type ConnectionReleaseCallback = (error: (Error|null), response?: database_driver_v1.ConnectionReleaseResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionGetInfo}.
         * @param error Error, if any
         * @param [response] ConnectionGetInfoResponse
         */
        type ConnectionGetInfoCallback = (error: (Error|null), response?: database_driver_v1.ConnectionGetInfoResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionGetObjects}.
         * @param error Error, if any
         * @param [response] ConnectionGetObjectsResponse
         */
        type ConnectionGetObjectsCallback = (error: (Error|null), response?: database_driver_v1.ConnectionGetObjectsResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionGetTableSchema}.
         * @param error Error, if any
         * @param [response] ConnectionGetTableSchemaResponse
         */
        type ConnectionGetTableSchemaCallback = (error: (Error|null), response?: database_driver_v1.ConnectionGetTableSchemaResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionGetTableTypes}.
         * @param error Error, if any
         * @param [response] ConnectionGetTableTypesResponse
         */
        type ConnectionGetTableTypesCallback = (error: (Error|null), response?: database_driver_v1.ConnectionGetTableTypesResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionCommit}.
         * @param error Error, if any
         * @param [response] ConnectionCommitResponse
         */
        type ConnectionCommitCallback = (error: (Error|null), response?: database_driver_v1.ConnectionCommitResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionRollback}.
         * @param error Error, if any
         * @param [response] ConnectionRollbackResponse
         */
        type ConnectionRollbackCallback = (error: (Error|null), response?: database_driver_v1.ConnectionRollbackResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionSetSessionParameters}.
         * @param error Error, if any
         * @param [response] ConnectionSetSessionParametersResponse
         */
        type ConnectionSetSessionParametersCallback = (error: (Error|null), response?: database_driver_v1.ConnectionSetSessionParametersResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#connectionGetParameter}.
         * @param error Error, if any
         * @param [response] ConnectionGetParameterResponse
         */
        type ConnectionGetParameterCallback = (error: (Error|null), response?: database_driver_v1.ConnectionGetParameterResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementNew}.
         * @param error Error, if any
         * @param [response] StatementNewResponse
         */
        type StatementNewCallback = (error: (Error|null), response?: database_driver_v1.StatementNewResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementRelease}.
         * @param error Error, if any
         * @param [response] StatementReleaseResponse
         */
        type StatementReleaseCallback = (error: (Error|null), response?: database_driver_v1.StatementReleaseResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementSetSqlQuery}.
         * @param error Error, if any
         * @param [response] StatementSetSqlQueryResponse
         */
        type StatementSetSqlQueryCallback = (error: (Error|null), response?: database_driver_v1.StatementSetSqlQueryResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementSetSubstraitPlan}.
         * @param error Error, if any
         * @param [response] StatementSetSubstraitPlanResponse
         */
        type StatementSetSubstraitPlanCallback = (error: (Error|null), response?: database_driver_v1.StatementSetSubstraitPlanResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementPrepare}.
         * @param error Error, if any
         * @param [response] StatementPrepareResponse
         */
        type StatementPrepareCallback = (error: (Error|null), response?: database_driver_v1.StatementPrepareResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementSetOptionString}.
         * @param error Error, if any
         * @param [response] StatementSetOptionStringResponse
         */
        type StatementSetOptionStringCallback = (error: (Error|null), response?: database_driver_v1.StatementSetOptionStringResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementSetOptionBytes}.
         * @param error Error, if any
         * @param [response] StatementSetOptionBytesResponse
         */
        type StatementSetOptionBytesCallback = (error: (Error|null), response?: database_driver_v1.StatementSetOptionBytesResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementSetOptionInt}.
         * @param error Error, if any
         * @param [response] StatementSetOptionIntResponse
         */
        type StatementSetOptionIntCallback = (error: (Error|null), response?: database_driver_v1.StatementSetOptionIntResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementSetOptionDouble}.
         * @param error Error, if any
         * @param [response] StatementSetOptionDoubleResponse
         */
        type StatementSetOptionDoubleCallback = (error: (Error|null), response?: database_driver_v1.StatementSetOptionDoubleResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementGetParameterSchema}.
         * @param error Error, if any
         * @param [response] StatementGetParameterSchemaResponse
         */
        type StatementGetParameterSchemaCallback = (error: (Error|null), response?: database_driver_v1.StatementGetParameterSchemaResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementExecuteQuery}.
         * @param error Error, if any
         * @param [response] StatementExecuteQueryResponse
         */
        type StatementExecuteQueryCallback = (error: (Error|null), response?: database_driver_v1.StatementExecuteQueryResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementExecutePartitions}.
         * @param error Error, if any
         * @param [response] StatementExecutePartitionsResponse
         */
        type StatementExecutePartitionsCallback = (error: (Error|null), response?: database_driver_v1.StatementExecutePartitionsResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#statementReadPartition}.
         * @param error Error, if any
         * @param [response] StatementReadPartitionResponse
         */
        type StatementReadPartitionCallback = (error: (Error|null), response?: database_driver_v1.StatementReadPartitionResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#configLoadAllSections}.
         * @param error Error, if any
         * @param [response] ConfigLoadAllSectionsResponse
         */
        type ConfigLoadAllSectionsCallback = (error: (Error|null), response?: database_driver_v1.ConfigLoadAllSectionsResponse) => void;

        /**
         * Callback as used by {@link database_driver_v1.DatabaseDriver#configGetPaths}.
         * @param error Error, if any
         * @param [response] ConfigGetPathsResponse
         */
        type ConfigGetPathsCallback = (error: (Error|null), response?: database_driver_v1.ConfigGetPathsResponse) => void;
    }
}

/** Namespace google. */
export namespace google {

    /** Namespace protobuf. */
    namespace protobuf {

        /** Properties of a FileDescriptorSet. */
        interface IFileDescriptorSet {

            /** FileDescriptorSet file */
            file?: (google.protobuf.IFileDescriptorProto[]|null);
        }

        /** Represents a FileDescriptorSet. */
        class FileDescriptorSet implements IFileDescriptorSet {

            /**
             * Constructs a new FileDescriptorSet.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFileDescriptorSet);

            /** FileDescriptorSet file. */
            public file: google.protobuf.IFileDescriptorProto[];

            /**
             * Creates a new FileDescriptorSet instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FileDescriptorSet instance
             */
            public static create(properties?: google.protobuf.IFileDescriptorSet): google.protobuf.FileDescriptorSet;

            /**
             * Encodes the specified FileDescriptorSet message. Does not implicitly {@link google.protobuf.FileDescriptorSet.verify|verify} messages.
             * @param message FileDescriptorSet message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFileDescriptorSet, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FileDescriptorSet message, length delimited. Does not implicitly {@link google.protobuf.FileDescriptorSet.verify|verify} messages.
             * @param message FileDescriptorSet message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFileDescriptorSet, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FileDescriptorSet message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FileDescriptorSet
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FileDescriptorSet;

            /**
             * Decodes a FileDescriptorSet message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FileDescriptorSet
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FileDescriptorSet;

            /**
             * Verifies a FileDescriptorSet message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FileDescriptorSet message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FileDescriptorSet
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FileDescriptorSet;

            /**
             * Creates a plain object from a FileDescriptorSet message. Also converts values to other types if specified.
             * @param message FileDescriptorSet
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FileDescriptorSet, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FileDescriptorSet to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FileDescriptorSet
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Edition enum. */
        enum Edition {
            EDITION_UNKNOWN = 0,
            EDITION_LEGACY = 900,
            EDITION_PROTO2 = 998,
            EDITION_PROTO3 = 999,
            EDITION_2023 = 1000,
            EDITION_2024 = 1001,
            EDITION_1_TEST_ONLY = 1,
            EDITION_2_TEST_ONLY = 2,
            EDITION_99997_TEST_ONLY = 99997,
            EDITION_99998_TEST_ONLY = 99998,
            EDITION_99999_TEST_ONLY = 99999,
            EDITION_MAX = 2147483647
        }

        /** Properties of a FileDescriptorProto. */
        interface IFileDescriptorProto {

            /** FileDescriptorProto name */
            name?: (string|null);

            /** FileDescriptorProto package */
            "package"?: (string|null);

            /** FileDescriptorProto dependency */
            dependency?: (string[]|null);

            /** FileDescriptorProto publicDependency */
            publicDependency?: (number[]|null);

            /** FileDescriptorProto weakDependency */
            weakDependency?: (number[]|null);

            /** FileDescriptorProto optionDependency */
            optionDependency?: (string[]|null);

            /** FileDescriptorProto messageType */
            messageType?: (google.protobuf.IDescriptorProto[]|null);

            /** FileDescriptorProto enumType */
            enumType?: (google.protobuf.IEnumDescriptorProto[]|null);

            /** FileDescriptorProto service */
            service?: (google.protobuf.IServiceDescriptorProto[]|null);

            /** FileDescriptorProto extension */
            extension?: (google.protobuf.IFieldDescriptorProto[]|null);

            /** FileDescriptorProto options */
            options?: (google.protobuf.IFileOptions|null);

            /** FileDescriptorProto sourceCodeInfo */
            sourceCodeInfo?: (google.protobuf.ISourceCodeInfo|null);

            /** FileDescriptorProto syntax */
            syntax?: (string|null);

            /** FileDescriptorProto edition */
            edition?: (google.protobuf.Edition|null);
        }

        /** Represents a FileDescriptorProto. */
        class FileDescriptorProto implements IFileDescriptorProto {

            /**
             * Constructs a new FileDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFileDescriptorProto);

            /** FileDescriptorProto name. */
            public name: string;

            /** FileDescriptorProto package. */
            public package: string;

            /** FileDescriptorProto dependency. */
            public dependency: string[];

            /** FileDescriptorProto publicDependency. */
            public publicDependency: number[];

            /** FileDescriptorProto weakDependency. */
            public weakDependency: number[];

            /** FileDescriptorProto optionDependency. */
            public optionDependency: string[];

            /** FileDescriptorProto messageType. */
            public messageType: google.protobuf.IDescriptorProto[];

            /** FileDescriptorProto enumType. */
            public enumType: google.protobuf.IEnumDescriptorProto[];

            /** FileDescriptorProto service. */
            public service: google.protobuf.IServiceDescriptorProto[];

            /** FileDescriptorProto extension. */
            public extension: google.protobuf.IFieldDescriptorProto[];

            /** FileDescriptorProto options. */
            public options?: (google.protobuf.IFileOptions|null);

            /** FileDescriptorProto sourceCodeInfo. */
            public sourceCodeInfo?: (google.protobuf.ISourceCodeInfo|null);

            /** FileDescriptorProto syntax. */
            public syntax: string;

            /** FileDescriptorProto edition. */
            public edition: google.protobuf.Edition;

            /**
             * Creates a new FileDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FileDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IFileDescriptorProto): google.protobuf.FileDescriptorProto;

            /**
             * Encodes the specified FileDescriptorProto message. Does not implicitly {@link google.protobuf.FileDescriptorProto.verify|verify} messages.
             * @param message FileDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFileDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FileDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.FileDescriptorProto.verify|verify} messages.
             * @param message FileDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFileDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FileDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FileDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FileDescriptorProto;

            /**
             * Decodes a FileDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FileDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FileDescriptorProto;

            /**
             * Verifies a FileDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FileDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FileDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FileDescriptorProto;

            /**
             * Creates a plain object from a FileDescriptorProto message. Also converts values to other types if specified.
             * @param message FileDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FileDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FileDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FileDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a DescriptorProto. */
        interface IDescriptorProto {

            /** DescriptorProto name */
            name?: (string|null);

            /** DescriptorProto field */
            field?: (google.protobuf.IFieldDescriptorProto[]|null);

            /** DescriptorProto extension */
            extension?: (google.protobuf.IFieldDescriptorProto[]|null);

            /** DescriptorProto nestedType */
            nestedType?: (google.protobuf.IDescriptorProto[]|null);

            /** DescriptorProto enumType */
            enumType?: (google.protobuf.IEnumDescriptorProto[]|null);

            /** DescriptorProto extensionRange */
            extensionRange?: (google.protobuf.DescriptorProto.IExtensionRange[]|null);

            /** DescriptorProto oneofDecl */
            oneofDecl?: (google.protobuf.IOneofDescriptorProto[]|null);

            /** DescriptorProto options */
            options?: (google.protobuf.IMessageOptions|null);

            /** DescriptorProto reservedRange */
            reservedRange?: (google.protobuf.DescriptorProto.IReservedRange[]|null);

            /** DescriptorProto reservedName */
            reservedName?: (string[]|null);

            /** DescriptorProto visibility */
            visibility?: (google.protobuf.SymbolVisibility|null);
        }

        /** Represents a DescriptorProto. */
        class DescriptorProto implements IDescriptorProto {

            /**
             * Constructs a new DescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IDescriptorProto);

            /** DescriptorProto name. */
            public name: string;

            /** DescriptorProto field. */
            public field: google.protobuf.IFieldDescriptorProto[];

            /** DescriptorProto extension. */
            public extension: google.protobuf.IFieldDescriptorProto[];

            /** DescriptorProto nestedType. */
            public nestedType: google.protobuf.IDescriptorProto[];

            /** DescriptorProto enumType. */
            public enumType: google.protobuf.IEnumDescriptorProto[];

            /** DescriptorProto extensionRange. */
            public extensionRange: google.protobuf.DescriptorProto.IExtensionRange[];

            /** DescriptorProto oneofDecl. */
            public oneofDecl: google.protobuf.IOneofDescriptorProto[];

            /** DescriptorProto options. */
            public options?: (google.protobuf.IMessageOptions|null);

            /** DescriptorProto reservedRange. */
            public reservedRange: google.protobuf.DescriptorProto.IReservedRange[];

            /** DescriptorProto reservedName. */
            public reservedName: string[];

            /** DescriptorProto visibility. */
            public visibility: google.protobuf.SymbolVisibility;

            /**
             * Creates a new DescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns DescriptorProto instance
             */
            public static create(properties?: google.protobuf.IDescriptorProto): google.protobuf.DescriptorProto;

            /**
             * Encodes the specified DescriptorProto message. Does not implicitly {@link google.protobuf.DescriptorProto.verify|verify} messages.
             * @param message DescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified DescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.DescriptorProto.verify|verify} messages.
             * @param message DescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a DescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns DescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.DescriptorProto;

            /**
             * Decodes a DescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns DescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.DescriptorProto;

            /**
             * Verifies a DescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a DescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns DescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.DescriptorProto;

            /**
             * Creates a plain object from a DescriptorProto message. Also converts values to other types if specified.
             * @param message DescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.DescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this DescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for DescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace DescriptorProto {

            /** Properties of an ExtensionRange. */
            interface IExtensionRange {

                /** ExtensionRange start */
                start?: (number|null);

                /** ExtensionRange end */
                end?: (number|null);

                /** ExtensionRange options */
                options?: (google.protobuf.IExtensionRangeOptions|null);
            }

            /** Represents an ExtensionRange. */
            class ExtensionRange implements IExtensionRange {

                /**
                 * Constructs a new ExtensionRange.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.DescriptorProto.IExtensionRange);

                /** ExtensionRange start. */
                public start: number;

                /** ExtensionRange end. */
                public end: number;

                /** ExtensionRange options. */
                public options?: (google.protobuf.IExtensionRangeOptions|null);

                /**
                 * Creates a new ExtensionRange instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ExtensionRange instance
                 */
                public static create(properties?: google.protobuf.DescriptorProto.IExtensionRange): google.protobuf.DescriptorProto.ExtensionRange;

                /**
                 * Encodes the specified ExtensionRange message. Does not implicitly {@link google.protobuf.DescriptorProto.ExtensionRange.verify|verify} messages.
                 * @param message ExtensionRange message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.DescriptorProto.IExtensionRange, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ExtensionRange message, length delimited. Does not implicitly {@link google.protobuf.DescriptorProto.ExtensionRange.verify|verify} messages.
                 * @param message ExtensionRange message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.DescriptorProto.IExtensionRange, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an ExtensionRange message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns ExtensionRange
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.DescriptorProto.ExtensionRange;

                /**
                 * Decodes an ExtensionRange message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns ExtensionRange
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.DescriptorProto.ExtensionRange;

                /**
                 * Verifies an ExtensionRange message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates an ExtensionRange message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ExtensionRange
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.DescriptorProto.ExtensionRange;

                /**
                 * Creates a plain object from an ExtensionRange message. Also converts values to other types if specified.
                 * @param message ExtensionRange
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.DescriptorProto.ExtensionRange, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ExtensionRange to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for ExtensionRange
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }

            /** Properties of a ReservedRange. */
            interface IReservedRange {

                /** ReservedRange start */
                start?: (number|null);

                /** ReservedRange end */
                end?: (number|null);
            }

            /** Represents a ReservedRange. */
            class ReservedRange implements IReservedRange {

                /**
                 * Constructs a new ReservedRange.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.DescriptorProto.IReservedRange);

                /** ReservedRange start. */
                public start: number;

                /** ReservedRange end. */
                public end: number;

                /**
                 * Creates a new ReservedRange instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns ReservedRange instance
                 */
                public static create(properties?: google.protobuf.DescriptorProto.IReservedRange): google.protobuf.DescriptorProto.ReservedRange;

                /**
                 * Encodes the specified ReservedRange message. Does not implicitly {@link google.protobuf.DescriptorProto.ReservedRange.verify|verify} messages.
                 * @param message ReservedRange message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.DescriptorProto.IReservedRange, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified ReservedRange message, length delimited. Does not implicitly {@link google.protobuf.DescriptorProto.ReservedRange.verify|verify} messages.
                 * @param message ReservedRange message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.DescriptorProto.IReservedRange, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a ReservedRange message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns ReservedRange
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.DescriptorProto.ReservedRange;

                /**
                 * Decodes a ReservedRange message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns ReservedRange
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.DescriptorProto.ReservedRange;

                /**
                 * Verifies a ReservedRange message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a ReservedRange message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ReservedRange
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.DescriptorProto.ReservedRange;

                /**
                 * Creates a plain object from a ReservedRange message. Also converts values to other types if specified.
                 * @param message ReservedRange
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.DescriptorProto.ReservedRange, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ReservedRange to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for ReservedRange
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }
        }

        /** Properties of an ExtensionRangeOptions. */
        interface IExtensionRangeOptions {

            /** ExtensionRangeOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);

            /** ExtensionRangeOptions declaration */
            declaration?: (google.protobuf.ExtensionRangeOptions.IDeclaration[]|null);

            /** ExtensionRangeOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** ExtensionRangeOptions verification */
            verification?: (google.protobuf.ExtensionRangeOptions.VerificationState|null);
        }

        /** Represents an ExtensionRangeOptions. */
        class ExtensionRangeOptions implements IExtensionRangeOptions {

            /**
             * Constructs a new ExtensionRangeOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IExtensionRangeOptions);

            /** ExtensionRangeOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /** ExtensionRangeOptions declaration. */
            public declaration: google.protobuf.ExtensionRangeOptions.IDeclaration[];

            /** ExtensionRangeOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** ExtensionRangeOptions verification. */
            public verification: google.protobuf.ExtensionRangeOptions.VerificationState;

            /**
             * Creates a new ExtensionRangeOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns ExtensionRangeOptions instance
             */
            public static create(properties?: google.protobuf.IExtensionRangeOptions): google.protobuf.ExtensionRangeOptions;

            /**
             * Encodes the specified ExtensionRangeOptions message. Does not implicitly {@link google.protobuf.ExtensionRangeOptions.verify|verify} messages.
             * @param message ExtensionRangeOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IExtensionRangeOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified ExtensionRangeOptions message, length delimited. Does not implicitly {@link google.protobuf.ExtensionRangeOptions.verify|verify} messages.
             * @param message ExtensionRangeOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IExtensionRangeOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an ExtensionRangeOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns ExtensionRangeOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.ExtensionRangeOptions;

            /**
             * Decodes an ExtensionRangeOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns ExtensionRangeOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.ExtensionRangeOptions;

            /**
             * Verifies an ExtensionRangeOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an ExtensionRangeOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns ExtensionRangeOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.ExtensionRangeOptions;

            /**
             * Creates a plain object from an ExtensionRangeOptions message. Also converts values to other types if specified.
             * @param message ExtensionRangeOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.ExtensionRangeOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this ExtensionRangeOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for ExtensionRangeOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace ExtensionRangeOptions {

            /** Properties of a Declaration. */
            interface IDeclaration {

                /** Declaration number */
                number?: (number|null);

                /** Declaration fullName */
                fullName?: (string|null);

                /** Declaration type */
                type?: (string|null);

                /** Declaration reserved */
                reserved?: (boolean|null);

                /** Declaration repeated */
                repeated?: (boolean|null);
            }

            /** Represents a Declaration. */
            class Declaration implements IDeclaration {

                /**
                 * Constructs a new Declaration.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.ExtensionRangeOptions.IDeclaration);

                /** Declaration number. */
                public number: number;

                /** Declaration fullName. */
                public fullName: string;

                /** Declaration type. */
                public type: string;

                /** Declaration reserved. */
                public reserved: boolean;

                /** Declaration repeated. */
                public repeated: boolean;

                /**
                 * Creates a new Declaration instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Declaration instance
                 */
                public static create(properties?: google.protobuf.ExtensionRangeOptions.IDeclaration): google.protobuf.ExtensionRangeOptions.Declaration;

                /**
                 * Encodes the specified Declaration message. Does not implicitly {@link google.protobuf.ExtensionRangeOptions.Declaration.verify|verify} messages.
                 * @param message Declaration message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.ExtensionRangeOptions.IDeclaration, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Declaration message, length delimited. Does not implicitly {@link google.protobuf.ExtensionRangeOptions.Declaration.verify|verify} messages.
                 * @param message Declaration message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.ExtensionRangeOptions.IDeclaration, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Declaration message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns Declaration
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.ExtensionRangeOptions.Declaration;

                /**
                 * Decodes a Declaration message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns Declaration
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.ExtensionRangeOptions.Declaration;

                /**
                 * Verifies a Declaration message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a Declaration message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Declaration
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.ExtensionRangeOptions.Declaration;

                /**
                 * Creates a plain object from a Declaration message. Also converts values to other types if specified.
                 * @param message Declaration
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.ExtensionRangeOptions.Declaration, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Declaration to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for Declaration
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }

            /** VerificationState enum. */
            enum VerificationState {
                DECLARATION = 0,
                UNVERIFIED = 1
            }
        }

        /** Properties of a FieldDescriptorProto. */
        interface IFieldDescriptorProto {

            /** FieldDescriptorProto name */
            name?: (string|null);

            /** FieldDescriptorProto number */
            number?: (number|null);

            /** FieldDescriptorProto label */
            label?: (google.protobuf.FieldDescriptorProto.Label|null);

            /** FieldDescriptorProto type */
            type?: (google.protobuf.FieldDescriptorProto.Type|null);

            /** FieldDescriptorProto typeName */
            typeName?: (string|null);

            /** FieldDescriptorProto extendee */
            extendee?: (string|null);

            /** FieldDescriptorProto defaultValue */
            defaultValue?: (string|null);

            /** FieldDescriptorProto oneofIndex */
            oneofIndex?: (number|null);

            /** FieldDescriptorProto jsonName */
            jsonName?: (string|null);

            /** FieldDescriptorProto options */
            options?: (google.protobuf.IFieldOptions|null);

            /** FieldDescriptorProto proto3Optional */
            proto3Optional?: (boolean|null);
        }

        /** Represents a FieldDescriptorProto. */
        class FieldDescriptorProto implements IFieldDescriptorProto {

            /**
             * Constructs a new FieldDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFieldDescriptorProto);

            /** FieldDescriptorProto name. */
            public name: string;

            /** FieldDescriptorProto number. */
            public number: number;

            /** FieldDescriptorProto label. */
            public label: google.protobuf.FieldDescriptorProto.Label;

            /** FieldDescriptorProto type. */
            public type: google.protobuf.FieldDescriptorProto.Type;

            /** FieldDescriptorProto typeName. */
            public typeName: string;

            /** FieldDescriptorProto extendee. */
            public extendee: string;

            /** FieldDescriptorProto defaultValue. */
            public defaultValue: string;

            /** FieldDescriptorProto oneofIndex. */
            public oneofIndex: number;

            /** FieldDescriptorProto jsonName. */
            public jsonName: string;

            /** FieldDescriptorProto options. */
            public options?: (google.protobuf.IFieldOptions|null);

            /** FieldDescriptorProto proto3Optional. */
            public proto3Optional: boolean;

            /**
             * Creates a new FieldDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FieldDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IFieldDescriptorProto): google.protobuf.FieldDescriptorProto;

            /**
             * Encodes the specified FieldDescriptorProto message. Does not implicitly {@link google.protobuf.FieldDescriptorProto.verify|verify} messages.
             * @param message FieldDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFieldDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FieldDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.FieldDescriptorProto.verify|verify} messages.
             * @param message FieldDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFieldDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FieldDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FieldDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FieldDescriptorProto;

            /**
             * Decodes a FieldDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FieldDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FieldDescriptorProto;

            /**
             * Verifies a FieldDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FieldDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FieldDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FieldDescriptorProto;

            /**
             * Creates a plain object from a FieldDescriptorProto message. Also converts values to other types if specified.
             * @param message FieldDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FieldDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FieldDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FieldDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace FieldDescriptorProto {

            /** Type enum. */
            enum Type {
                TYPE_DOUBLE = 1,
                TYPE_FLOAT = 2,
                TYPE_INT64 = 3,
                TYPE_UINT64 = 4,
                TYPE_INT32 = 5,
                TYPE_FIXED64 = 6,
                TYPE_FIXED32 = 7,
                TYPE_BOOL = 8,
                TYPE_STRING = 9,
                TYPE_GROUP = 10,
                TYPE_MESSAGE = 11,
                TYPE_BYTES = 12,
                TYPE_UINT32 = 13,
                TYPE_ENUM = 14,
                TYPE_SFIXED32 = 15,
                TYPE_SFIXED64 = 16,
                TYPE_SINT32 = 17,
                TYPE_SINT64 = 18
            }

            /** Label enum. */
            enum Label {
                LABEL_OPTIONAL = 1,
                LABEL_REPEATED = 3,
                LABEL_REQUIRED = 2
            }
        }

        /** Properties of an OneofDescriptorProto. */
        interface IOneofDescriptorProto {

            /** OneofDescriptorProto name */
            name?: (string|null);

            /** OneofDescriptorProto options */
            options?: (google.protobuf.IOneofOptions|null);
        }

        /** Represents an OneofDescriptorProto. */
        class OneofDescriptorProto implements IOneofDescriptorProto {

            /**
             * Constructs a new OneofDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IOneofDescriptorProto);

            /** OneofDescriptorProto name. */
            public name: string;

            /** OneofDescriptorProto options. */
            public options?: (google.protobuf.IOneofOptions|null);

            /**
             * Creates a new OneofDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns OneofDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IOneofDescriptorProto): google.protobuf.OneofDescriptorProto;

            /**
             * Encodes the specified OneofDescriptorProto message. Does not implicitly {@link google.protobuf.OneofDescriptorProto.verify|verify} messages.
             * @param message OneofDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IOneofDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified OneofDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.OneofDescriptorProto.verify|verify} messages.
             * @param message OneofDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IOneofDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an OneofDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns OneofDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.OneofDescriptorProto;

            /**
             * Decodes an OneofDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns OneofDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.OneofDescriptorProto;

            /**
             * Verifies an OneofDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an OneofDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns OneofDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.OneofDescriptorProto;

            /**
             * Creates a plain object from an OneofDescriptorProto message. Also converts values to other types if specified.
             * @param message OneofDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.OneofDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this OneofDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for OneofDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of an EnumDescriptorProto. */
        interface IEnumDescriptorProto {

            /** EnumDescriptorProto name */
            name?: (string|null);

            /** EnumDescriptorProto value */
            value?: (google.protobuf.IEnumValueDescriptorProto[]|null);

            /** EnumDescriptorProto options */
            options?: (google.protobuf.IEnumOptions|null);

            /** EnumDescriptorProto reservedRange */
            reservedRange?: (google.protobuf.EnumDescriptorProto.IEnumReservedRange[]|null);

            /** EnumDescriptorProto reservedName */
            reservedName?: (string[]|null);

            /** EnumDescriptorProto visibility */
            visibility?: (google.protobuf.SymbolVisibility|null);
        }

        /** Represents an EnumDescriptorProto. */
        class EnumDescriptorProto implements IEnumDescriptorProto {

            /**
             * Constructs a new EnumDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IEnumDescriptorProto);

            /** EnumDescriptorProto name. */
            public name: string;

            /** EnumDescriptorProto value. */
            public value: google.protobuf.IEnumValueDescriptorProto[];

            /** EnumDescriptorProto options. */
            public options?: (google.protobuf.IEnumOptions|null);

            /** EnumDescriptorProto reservedRange. */
            public reservedRange: google.protobuf.EnumDescriptorProto.IEnumReservedRange[];

            /** EnumDescriptorProto reservedName. */
            public reservedName: string[];

            /** EnumDescriptorProto visibility. */
            public visibility: google.protobuf.SymbolVisibility;

            /**
             * Creates a new EnumDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns EnumDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IEnumDescriptorProto): google.protobuf.EnumDescriptorProto;

            /**
             * Encodes the specified EnumDescriptorProto message. Does not implicitly {@link google.protobuf.EnumDescriptorProto.verify|verify} messages.
             * @param message EnumDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IEnumDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified EnumDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.EnumDescriptorProto.verify|verify} messages.
             * @param message EnumDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IEnumDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an EnumDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns EnumDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.EnumDescriptorProto;

            /**
             * Decodes an EnumDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns EnumDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.EnumDescriptorProto;

            /**
             * Verifies an EnumDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an EnumDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns EnumDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.EnumDescriptorProto;

            /**
             * Creates a plain object from an EnumDescriptorProto message. Also converts values to other types if specified.
             * @param message EnumDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.EnumDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this EnumDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for EnumDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace EnumDescriptorProto {

            /** Properties of an EnumReservedRange. */
            interface IEnumReservedRange {

                /** EnumReservedRange start */
                start?: (number|null);

                /** EnumReservedRange end */
                end?: (number|null);
            }

            /** Represents an EnumReservedRange. */
            class EnumReservedRange implements IEnumReservedRange {

                /**
                 * Constructs a new EnumReservedRange.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.EnumDescriptorProto.IEnumReservedRange);

                /** EnumReservedRange start. */
                public start: number;

                /** EnumReservedRange end. */
                public end: number;

                /**
                 * Creates a new EnumReservedRange instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns EnumReservedRange instance
                 */
                public static create(properties?: google.protobuf.EnumDescriptorProto.IEnumReservedRange): google.protobuf.EnumDescriptorProto.EnumReservedRange;

                /**
                 * Encodes the specified EnumReservedRange message. Does not implicitly {@link google.protobuf.EnumDescriptorProto.EnumReservedRange.verify|verify} messages.
                 * @param message EnumReservedRange message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.EnumDescriptorProto.IEnumReservedRange, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified EnumReservedRange message, length delimited. Does not implicitly {@link google.protobuf.EnumDescriptorProto.EnumReservedRange.verify|verify} messages.
                 * @param message EnumReservedRange message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.EnumDescriptorProto.IEnumReservedRange, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an EnumReservedRange message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns EnumReservedRange
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.EnumDescriptorProto.EnumReservedRange;

                /**
                 * Decodes an EnumReservedRange message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns EnumReservedRange
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.EnumDescriptorProto.EnumReservedRange;

                /**
                 * Verifies an EnumReservedRange message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates an EnumReservedRange message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns EnumReservedRange
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.EnumDescriptorProto.EnumReservedRange;

                /**
                 * Creates a plain object from an EnumReservedRange message. Also converts values to other types if specified.
                 * @param message EnumReservedRange
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.EnumDescriptorProto.EnumReservedRange, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this EnumReservedRange to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for EnumReservedRange
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }
        }

        /** Properties of an EnumValueDescriptorProto. */
        interface IEnumValueDescriptorProto {

            /** EnumValueDescriptorProto name */
            name?: (string|null);

            /** EnumValueDescriptorProto number */
            number?: (number|null);

            /** EnumValueDescriptorProto options */
            options?: (google.protobuf.IEnumValueOptions|null);
        }

        /** Represents an EnumValueDescriptorProto. */
        class EnumValueDescriptorProto implements IEnumValueDescriptorProto {

            /**
             * Constructs a new EnumValueDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IEnumValueDescriptorProto);

            /** EnumValueDescriptorProto name. */
            public name: string;

            /** EnumValueDescriptorProto number. */
            public number: number;

            /** EnumValueDescriptorProto options. */
            public options?: (google.protobuf.IEnumValueOptions|null);

            /**
             * Creates a new EnumValueDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns EnumValueDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IEnumValueDescriptorProto): google.protobuf.EnumValueDescriptorProto;

            /**
             * Encodes the specified EnumValueDescriptorProto message. Does not implicitly {@link google.protobuf.EnumValueDescriptorProto.verify|verify} messages.
             * @param message EnumValueDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IEnumValueDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified EnumValueDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.EnumValueDescriptorProto.verify|verify} messages.
             * @param message EnumValueDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IEnumValueDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an EnumValueDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns EnumValueDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.EnumValueDescriptorProto;

            /**
             * Decodes an EnumValueDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns EnumValueDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.EnumValueDescriptorProto;

            /**
             * Verifies an EnumValueDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an EnumValueDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns EnumValueDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.EnumValueDescriptorProto;

            /**
             * Creates a plain object from an EnumValueDescriptorProto message. Also converts values to other types if specified.
             * @param message EnumValueDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.EnumValueDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this EnumValueDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for EnumValueDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a ServiceDescriptorProto. */
        interface IServiceDescriptorProto {

            /** ServiceDescriptorProto name */
            name?: (string|null);

            /** ServiceDescriptorProto method */
            method?: (google.protobuf.IMethodDescriptorProto[]|null);

            /** ServiceDescriptorProto options */
            options?: (google.protobuf.IServiceOptions|null);
        }

        /** Represents a ServiceDescriptorProto. */
        class ServiceDescriptorProto implements IServiceDescriptorProto {

            /**
             * Constructs a new ServiceDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IServiceDescriptorProto);

            /** ServiceDescriptorProto name. */
            public name: string;

            /** ServiceDescriptorProto method. */
            public method: google.protobuf.IMethodDescriptorProto[];

            /** ServiceDescriptorProto options. */
            public options?: (google.protobuf.IServiceOptions|null);

            /**
             * Creates a new ServiceDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns ServiceDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IServiceDescriptorProto): google.protobuf.ServiceDescriptorProto;

            /**
             * Encodes the specified ServiceDescriptorProto message. Does not implicitly {@link google.protobuf.ServiceDescriptorProto.verify|verify} messages.
             * @param message ServiceDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IServiceDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified ServiceDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.ServiceDescriptorProto.verify|verify} messages.
             * @param message ServiceDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IServiceDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a ServiceDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns ServiceDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.ServiceDescriptorProto;

            /**
             * Decodes a ServiceDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns ServiceDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.ServiceDescriptorProto;

            /**
             * Verifies a ServiceDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a ServiceDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns ServiceDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.ServiceDescriptorProto;

            /**
             * Creates a plain object from a ServiceDescriptorProto message. Also converts values to other types if specified.
             * @param message ServiceDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.ServiceDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this ServiceDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for ServiceDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a MethodDescriptorProto. */
        interface IMethodDescriptorProto {

            /** MethodDescriptorProto name */
            name?: (string|null);

            /** MethodDescriptorProto inputType */
            inputType?: (string|null);

            /** MethodDescriptorProto outputType */
            outputType?: (string|null);

            /** MethodDescriptorProto options */
            options?: (google.protobuf.IMethodOptions|null);

            /** MethodDescriptorProto clientStreaming */
            clientStreaming?: (boolean|null);

            /** MethodDescriptorProto serverStreaming */
            serverStreaming?: (boolean|null);
        }

        /** Represents a MethodDescriptorProto. */
        class MethodDescriptorProto implements IMethodDescriptorProto {

            /**
             * Constructs a new MethodDescriptorProto.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IMethodDescriptorProto);

            /** MethodDescriptorProto name. */
            public name: string;

            /** MethodDescriptorProto inputType. */
            public inputType: string;

            /** MethodDescriptorProto outputType. */
            public outputType: string;

            /** MethodDescriptorProto options. */
            public options?: (google.protobuf.IMethodOptions|null);

            /** MethodDescriptorProto clientStreaming. */
            public clientStreaming: boolean;

            /** MethodDescriptorProto serverStreaming. */
            public serverStreaming: boolean;

            /**
             * Creates a new MethodDescriptorProto instance using the specified properties.
             * @param [properties] Properties to set
             * @returns MethodDescriptorProto instance
             */
            public static create(properties?: google.protobuf.IMethodDescriptorProto): google.protobuf.MethodDescriptorProto;

            /**
             * Encodes the specified MethodDescriptorProto message. Does not implicitly {@link google.protobuf.MethodDescriptorProto.verify|verify} messages.
             * @param message MethodDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IMethodDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified MethodDescriptorProto message, length delimited. Does not implicitly {@link google.protobuf.MethodDescriptorProto.verify|verify} messages.
             * @param message MethodDescriptorProto message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IMethodDescriptorProto, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a MethodDescriptorProto message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns MethodDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.MethodDescriptorProto;

            /**
             * Decodes a MethodDescriptorProto message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns MethodDescriptorProto
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.MethodDescriptorProto;

            /**
             * Verifies a MethodDescriptorProto message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a MethodDescriptorProto message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns MethodDescriptorProto
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.MethodDescriptorProto;

            /**
             * Creates a plain object from a MethodDescriptorProto message. Also converts values to other types if specified.
             * @param message MethodDescriptorProto
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.MethodDescriptorProto, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this MethodDescriptorProto to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for MethodDescriptorProto
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a FileOptions. */
        interface IFileOptions {

            /** FileOptions javaPackage */
            javaPackage?: (string|null);

            /** FileOptions javaOuterClassname */
            javaOuterClassname?: (string|null);

            /** FileOptions javaMultipleFiles */
            javaMultipleFiles?: (boolean|null);

            /** FileOptions javaGenerateEqualsAndHash */
            javaGenerateEqualsAndHash?: (boolean|null);

            /** FileOptions javaStringCheckUtf8 */
            javaStringCheckUtf8?: (boolean|null);

            /** FileOptions optimizeFor */
            optimizeFor?: (google.protobuf.FileOptions.OptimizeMode|null);

            /** FileOptions goPackage */
            goPackage?: (string|null);

            /** FileOptions ccGenericServices */
            ccGenericServices?: (boolean|null);

            /** FileOptions javaGenericServices */
            javaGenericServices?: (boolean|null);

            /** FileOptions pyGenericServices */
            pyGenericServices?: (boolean|null);

            /** FileOptions deprecated */
            deprecated?: (boolean|null);

            /** FileOptions ccEnableArenas */
            ccEnableArenas?: (boolean|null);

            /** FileOptions objcClassPrefix */
            objcClassPrefix?: (string|null);

            /** FileOptions csharpNamespace */
            csharpNamespace?: (string|null);

            /** FileOptions swiftPrefix */
            swiftPrefix?: (string|null);

            /** FileOptions phpClassPrefix */
            phpClassPrefix?: (string|null);

            /** FileOptions phpNamespace */
            phpNamespace?: (string|null);

            /** FileOptions phpMetadataNamespace */
            phpMetadataNamespace?: (string|null);

            /** FileOptions rubyPackage */
            rubyPackage?: (string|null);

            /** FileOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** FileOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);
        }

        /** Represents a FileOptions. */
        class FileOptions implements IFileOptions {

            /**
             * Constructs a new FileOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFileOptions);

            /** FileOptions javaPackage. */
            public javaPackage: string;

            /** FileOptions javaOuterClassname. */
            public javaOuterClassname: string;

            /** FileOptions javaMultipleFiles. */
            public javaMultipleFiles: boolean;

            /** FileOptions javaGenerateEqualsAndHash. */
            public javaGenerateEqualsAndHash: boolean;

            /** FileOptions javaStringCheckUtf8. */
            public javaStringCheckUtf8: boolean;

            /** FileOptions optimizeFor. */
            public optimizeFor: google.protobuf.FileOptions.OptimizeMode;

            /** FileOptions goPackage. */
            public goPackage: string;

            /** FileOptions ccGenericServices. */
            public ccGenericServices: boolean;

            /** FileOptions javaGenericServices. */
            public javaGenericServices: boolean;

            /** FileOptions pyGenericServices. */
            public pyGenericServices: boolean;

            /** FileOptions deprecated. */
            public deprecated: boolean;

            /** FileOptions ccEnableArenas. */
            public ccEnableArenas: boolean;

            /** FileOptions objcClassPrefix. */
            public objcClassPrefix: string;

            /** FileOptions csharpNamespace. */
            public csharpNamespace: string;

            /** FileOptions swiftPrefix. */
            public swiftPrefix: string;

            /** FileOptions phpClassPrefix. */
            public phpClassPrefix: string;

            /** FileOptions phpNamespace. */
            public phpNamespace: string;

            /** FileOptions phpMetadataNamespace. */
            public phpMetadataNamespace: string;

            /** FileOptions rubyPackage. */
            public rubyPackage: string;

            /** FileOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** FileOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new FileOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FileOptions instance
             */
            public static create(properties?: google.protobuf.IFileOptions): google.protobuf.FileOptions;

            /**
             * Encodes the specified FileOptions message. Does not implicitly {@link google.protobuf.FileOptions.verify|verify} messages.
             * @param message FileOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFileOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FileOptions message, length delimited. Does not implicitly {@link google.protobuf.FileOptions.verify|verify} messages.
             * @param message FileOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFileOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FileOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FileOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FileOptions;

            /**
             * Decodes a FileOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FileOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FileOptions;

            /**
             * Verifies a FileOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FileOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FileOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FileOptions;

            /**
             * Creates a plain object from a FileOptions message. Also converts values to other types if specified.
             * @param message FileOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FileOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FileOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FileOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace FileOptions {

            /** OptimizeMode enum. */
            enum OptimizeMode {
                SPEED = 1,
                CODE_SIZE = 2,
                LITE_RUNTIME = 3
            }
        }

        /** Properties of a MessageOptions. */
        interface IMessageOptions {

            /** MessageOptions messageSetWireFormat */
            messageSetWireFormat?: (boolean|null);

            /** MessageOptions noStandardDescriptorAccessor */
            noStandardDescriptorAccessor?: (boolean|null);

            /** MessageOptions deprecated */
            deprecated?: (boolean|null);

            /** MessageOptions mapEntry */
            mapEntry?: (boolean|null);

            /** MessageOptions deprecatedLegacyJsonFieldConflicts */
            deprecatedLegacyJsonFieldConflicts?: (boolean|null);

            /** MessageOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** MessageOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);
        }

        /** Represents a MessageOptions. */
        class MessageOptions implements IMessageOptions {

            /**
             * Constructs a new MessageOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IMessageOptions);

            /** MessageOptions messageSetWireFormat. */
            public messageSetWireFormat: boolean;

            /** MessageOptions noStandardDescriptorAccessor. */
            public noStandardDescriptorAccessor: boolean;

            /** MessageOptions deprecated. */
            public deprecated: boolean;

            /** MessageOptions mapEntry. */
            public mapEntry: boolean;

            /** MessageOptions deprecatedLegacyJsonFieldConflicts. */
            public deprecatedLegacyJsonFieldConflicts: boolean;

            /** MessageOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** MessageOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new MessageOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns MessageOptions instance
             */
            public static create(properties?: google.protobuf.IMessageOptions): google.protobuf.MessageOptions;

            /**
             * Encodes the specified MessageOptions message. Does not implicitly {@link google.protobuf.MessageOptions.verify|verify} messages.
             * @param message MessageOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IMessageOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified MessageOptions message, length delimited. Does not implicitly {@link google.protobuf.MessageOptions.verify|verify} messages.
             * @param message MessageOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IMessageOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a MessageOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns MessageOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.MessageOptions;

            /**
             * Decodes a MessageOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns MessageOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.MessageOptions;

            /**
             * Verifies a MessageOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a MessageOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns MessageOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.MessageOptions;

            /**
             * Creates a plain object from a MessageOptions message. Also converts values to other types if specified.
             * @param message MessageOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.MessageOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this MessageOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for MessageOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a FieldOptions. */
        interface IFieldOptions {

            /** FieldOptions ctype */
            ctype?: (google.protobuf.FieldOptions.CType|null);

            /** FieldOptions packed */
            packed?: (boolean|null);

            /** FieldOptions jstype */
            jstype?: (google.protobuf.FieldOptions.JSType|null);

            /** FieldOptions lazy */
            lazy?: (boolean|null);

            /** FieldOptions unverifiedLazy */
            unverifiedLazy?: (boolean|null);

            /** FieldOptions deprecated */
            deprecated?: (boolean|null);

            /** FieldOptions weak */
            weak?: (boolean|null);

            /** FieldOptions debugRedact */
            debugRedact?: (boolean|null);

            /** FieldOptions retention */
            retention?: (google.protobuf.FieldOptions.OptionRetention|null);

            /** FieldOptions targets */
            targets?: (google.protobuf.FieldOptions.OptionTargetType[]|null);

            /** FieldOptions editionDefaults */
            editionDefaults?: (google.protobuf.FieldOptions.IEditionDefault[]|null);

            /** FieldOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** FieldOptions featureSupport */
            featureSupport?: (google.protobuf.FieldOptions.IFeatureSupport|null);

            /** FieldOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);
        }

        /** Represents a FieldOptions. */
        class FieldOptions implements IFieldOptions {

            /**
             * Constructs a new FieldOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFieldOptions);

            /** FieldOptions ctype. */
            public ctype: google.protobuf.FieldOptions.CType;

            /** FieldOptions packed. */
            public packed: boolean;

            /** FieldOptions jstype. */
            public jstype: google.protobuf.FieldOptions.JSType;

            /** FieldOptions lazy. */
            public lazy: boolean;

            /** FieldOptions unverifiedLazy. */
            public unverifiedLazy: boolean;

            /** FieldOptions deprecated. */
            public deprecated: boolean;

            /** FieldOptions weak. */
            public weak: boolean;

            /** FieldOptions debugRedact. */
            public debugRedact: boolean;

            /** FieldOptions retention. */
            public retention: google.protobuf.FieldOptions.OptionRetention;

            /** FieldOptions targets. */
            public targets: google.protobuf.FieldOptions.OptionTargetType[];

            /** FieldOptions editionDefaults. */
            public editionDefaults: google.protobuf.FieldOptions.IEditionDefault[];

            /** FieldOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** FieldOptions featureSupport. */
            public featureSupport?: (google.protobuf.FieldOptions.IFeatureSupport|null);

            /** FieldOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new FieldOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FieldOptions instance
             */
            public static create(properties?: google.protobuf.IFieldOptions): google.protobuf.FieldOptions;

            /**
             * Encodes the specified FieldOptions message. Does not implicitly {@link google.protobuf.FieldOptions.verify|verify} messages.
             * @param message FieldOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFieldOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FieldOptions message, length delimited. Does not implicitly {@link google.protobuf.FieldOptions.verify|verify} messages.
             * @param message FieldOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFieldOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FieldOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FieldOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FieldOptions;

            /**
             * Decodes a FieldOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FieldOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FieldOptions;

            /**
             * Verifies a FieldOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FieldOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FieldOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FieldOptions;

            /**
             * Creates a plain object from a FieldOptions message. Also converts values to other types if specified.
             * @param message FieldOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FieldOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FieldOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FieldOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace FieldOptions {

            /** CType enum. */
            enum CType {
                STRING = 0,
                CORD = 1,
                STRING_PIECE = 2
            }

            /** JSType enum. */
            enum JSType {
                JS_NORMAL = 0,
                JS_STRING = 1,
                JS_NUMBER = 2
            }

            /** OptionRetention enum. */
            enum OptionRetention {
                RETENTION_UNKNOWN = 0,
                RETENTION_RUNTIME = 1,
                RETENTION_SOURCE = 2
            }

            /** OptionTargetType enum. */
            enum OptionTargetType {
                TARGET_TYPE_UNKNOWN = 0,
                TARGET_TYPE_FILE = 1,
                TARGET_TYPE_EXTENSION_RANGE = 2,
                TARGET_TYPE_MESSAGE = 3,
                TARGET_TYPE_FIELD = 4,
                TARGET_TYPE_ONEOF = 5,
                TARGET_TYPE_ENUM = 6,
                TARGET_TYPE_ENUM_ENTRY = 7,
                TARGET_TYPE_SERVICE = 8,
                TARGET_TYPE_METHOD = 9
            }

            /** Properties of an EditionDefault. */
            interface IEditionDefault {

                /** EditionDefault edition */
                edition?: (google.protobuf.Edition|null);

                /** EditionDefault value */
                value?: (string|null);
            }

            /** Represents an EditionDefault. */
            class EditionDefault implements IEditionDefault {

                /**
                 * Constructs a new EditionDefault.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.FieldOptions.IEditionDefault);

                /** EditionDefault edition. */
                public edition: google.protobuf.Edition;

                /** EditionDefault value. */
                public value: string;

                /**
                 * Creates a new EditionDefault instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns EditionDefault instance
                 */
                public static create(properties?: google.protobuf.FieldOptions.IEditionDefault): google.protobuf.FieldOptions.EditionDefault;

                /**
                 * Encodes the specified EditionDefault message. Does not implicitly {@link google.protobuf.FieldOptions.EditionDefault.verify|verify} messages.
                 * @param message EditionDefault message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.FieldOptions.IEditionDefault, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified EditionDefault message, length delimited. Does not implicitly {@link google.protobuf.FieldOptions.EditionDefault.verify|verify} messages.
                 * @param message EditionDefault message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.FieldOptions.IEditionDefault, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an EditionDefault message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns EditionDefault
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FieldOptions.EditionDefault;

                /**
                 * Decodes an EditionDefault message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns EditionDefault
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FieldOptions.EditionDefault;

                /**
                 * Verifies an EditionDefault message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates an EditionDefault message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns EditionDefault
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.FieldOptions.EditionDefault;

                /**
                 * Creates a plain object from an EditionDefault message. Also converts values to other types if specified.
                 * @param message EditionDefault
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.FieldOptions.EditionDefault, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this EditionDefault to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for EditionDefault
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }

            /** Properties of a FeatureSupport. */
            interface IFeatureSupport {

                /** FeatureSupport editionIntroduced */
                editionIntroduced?: (google.protobuf.Edition|null);

                /** FeatureSupport editionDeprecated */
                editionDeprecated?: (google.protobuf.Edition|null);

                /** FeatureSupport deprecationWarning */
                deprecationWarning?: (string|null);

                /** FeatureSupport editionRemoved */
                editionRemoved?: (google.protobuf.Edition|null);
            }

            /** Represents a FeatureSupport. */
            class FeatureSupport implements IFeatureSupport {

                /**
                 * Constructs a new FeatureSupport.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.FieldOptions.IFeatureSupport);

                /** FeatureSupport editionIntroduced. */
                public editionIntroduced: google.protobuf.Edition;

                /** FeatureSupport editionDeprecated. */
                public editionDeprecated: google.protobuf.Edition;

                /** FeatureSupport deprecationWarning. */
                public deprecationWarning: string;

                /** FeatureSupport editionRemoved. */
                public editionRemoved: google.protobuf.Edition;

                /**
                 * Creates a new FeatureSupport instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns FeatureSupport instance
                 */
                public static create(properties?: google.protobuf.FieldOptions.IFeatureSupport): google.protobuf.FieldOptions.FeatureSupport;

                /**
                 * Encodes the specified FeatureSupport message. Does not implicitly {@link google.protobuf.FieldOptions.FeatureSupport.verify|verify} messages.
                 * @param message FeatureSupport message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.FieldOptions.IFeatureSupport, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified FeatureSupport message, length delimited. Does not implicitly {@link google.protobuf.FieldOptions.FeatureSupport.verify|verify} messages.
                 * @param message FeatureSupport message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.FieldOptions.IFeatureSupport, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a FeatureSupport message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns FeatureSupport
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FieldOptions.FeatureSupport;

                /**
                 * Decodes a FeatureSupport message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns FeatureSupport
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FieldOptions.FeatureSupport;

                /**
                 * Verifies a FeatureSupport message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a FeatureSupport message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns FeatureSupport
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.FieldOptions.FeatureSupport;

                /**
                 * Creates a plain object from a FeatureSupport message. Also converts values to other types if specified.
                 * @param message FeatureSupport
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.FieldOptions.FeatureSupport, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this FeatureSupport to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for FeatureSupport
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }
        }

        /** Properties of an OneofOptions. */
        interface IOneofOptions {

            /** OneofOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** OneofOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);
        }

        /** Represents an OneofOptions. */
        class OneofOptions implements IOneofOptions {

            /**
             * Constructs a new OneofOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IOneofOptions);

            /** OneofOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** OneofOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new OneofOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns OneofOptions instance
             */
            public static create(properties?: google.protobuf.IOneofOptions): google.protobuf.OneofOptions;

            /**
             * Encodes the specified OneofOptions message. Does not implicitly {@link google.protobuf.OneofOptions.verify|verify} messages.
             * @param message OneofOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IOneofOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified OneofOptions message, length delimited. Does not implicitly {@link google.protobuf.OneofOptions.verify|verify} messages.
             * @param message OneofOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IOneofOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an OneofOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns OneofOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.OneofOptions;

            /**
             * Decodes an OneofOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns OneofOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.OneofOptions;

            /**
             * Verifies an OneofOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an OneofOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns OneofOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.OneofOptions;

            /**
             * Creates a plain object from an OneofOptions message. Also converts values to other types if specified.
             * @param message OneofOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.OneofOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this OneofOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for OneofOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of an EnumOptions. */
        interface IEnumOptions {

            /** EnumOptions allowAlias */
            allowAlias?: (boolean|null);

            /** EnumOptions deprecated */
            deprecated?: (boolean|null);

            /** EnumOptions deprecatedLegacyJsonFieldConflicts */
            deprecatedLegacyJsonFieldConflicts?: (boolean|null);

            /** EnumOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** EnumOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);
        }

        /** Represents an EnumOptions. */
        class EnumOptions implements IEnumOptions {

            /**
             * Constructs a new EnumOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IEnumOptions);

            /** EnumOptions allowAlias. */
            public allowAlias: boolean;

            /** EnumOptions deprecated. */
            public deprecated: boolean;

            /** EnumOptions deprecatedLegacyJsonFieldConflicts. */
            public deprecatedLegacyJsonFieldConflicts: boolean;

            /** EnumOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** EnumOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new EnumOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns EnumOptions instance
             */
            public static create(properties?: google.protobuf.IEnumOptions): google.protobuf.EnumOptions;

            /**
             * Encodes the specified EnumOptions message. Does not implicitly {@link google.protobuf.EnumOptions.verify|verify} messages.
             * @param message EnumOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IEnumOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified EnumOptions message, length delimited. Does not implicitly {@link google.protobuf.EnumOptions.verify|verify} messages.
             * @param message EnumOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IEnumOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an EnumOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns EnumOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.EnumOptions;

            /**
             * Decodes an EnumOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns EnumOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.EnumOptions;

            /**
             * Verifies an EnumOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an EnumOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns EnumOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.EnumOptions;

            /**
             * Creates a plain object from an EnumOptions message. Also converts values to other types if specified.
             * @param message EnumOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.EnumOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this EnumOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for EnumOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of an EnumValueOptions. */
        interface IEnumValueOptions {

            /** EnumValueOptions deprecated */
            deprecated?: (boolean|null);

            /** EnumValueOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** EnumValueOptions debugRedact */
            debugRedact?: (boolean|null);

            /** EnumValueOptions featureSupport */
            featureSupport?: (google.protobuf.FieldOptions.IFeatureSupport|null);

            /** EnumValueOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);
        }

        /** Represents an EnumValueOptions. */
        class EnumValueOptions implements IEnumValueOptions {

            /**
             * Constructs a new EnumValueOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IEnumValueOptions);

            /** EnumValueOptions deprecated. */
            public deprecated: boolean;

            /** EnumValueOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** EnumValueOptions debugRedact. */
            public debugRedact: boolean;

            /** EnumValueOptions featureSupport. */
            public featureSupport?: (google.protobuf.FieldOptions.IFeatureSupport|null);

            /** EnumValueOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new EnumValueOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns EnumValueOptions instance
             */
            public static create(properties?: google.protobuf.IEnumValueOptions): google.protobuf.EnumValueOptions;

            /**
             * Encodes the specified EnumValueOptions message. Does not implicitly {@link google.protobuf.EnumValueOptions.verify|verify} messages.
             * @param message EnumValueOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IEnumValueOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified EnumValueOptions message, length delimited. Does not implicitly {@link google.protobuf.EnumValueOptions.verify|verify} messages.
             * @param message EnumValueOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IEnumValueOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an EnumValueOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns EnumValueOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.EnumValueOptions;

            /**
             * Decodes an EnumValueOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns EnumValueOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.EnumValueOptions;

            /**
             * Verifies an EnumValueOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an EnumValueOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns EnumValueOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.EnumValueOptions;

            /**
             * Creates a plain object from an EnumValueOptions message. Also converts values to other types if specified.
             * @param message EnumValueOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.EnumValueOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this EnumValueOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for EnumValueOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a ServiceOptions. */
        interface IServiceOptions {

            /** ServiceOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** ServiceOptions deprecated */
            deprecated?: (boolean|null);

            /** ServiceOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);

            /** ServiceOptions .database_driver_v1.serviceError */
            ".database_driver_v1.serviceError"?: (string|null);
        }

        /** Represents a ServiceOptions. */
        class ServiceOptions implements IServiceOptions {

            /**
             * Constructs a new ServiceOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IServiceOptions);

            /** ServiceOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** ServiceOptions deprecated. */
            public deprecated: boolean;

            /** ServiceOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new ServiceOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns ServiceOptions instance
             */
            public static create(properties?: google.protobuf.IServiceOptions): google.protobuf.ServiceOptions;

            /**
             * Encodes the specified ServiceOptions message. Does not implicitly {@link google.protobuf.ServiceOptions.verify|verify} messages.
             * @param message ServiceOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IServiceOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified ServiceOptions message, length delimited. Does not implicitly {@link google.protobuf.ServiceOptions.verify|verify} messages.
             * @param message ServiceOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IServiceOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a ServiceOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns ServiceOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.ServiceOptions;

            /**
             * Decodes a ServiceOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns ServiceOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.ServiceOptions;

            /**
             * Verifies a ServiceOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a ServiceOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns ServiceOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.ServiceOptions;

            /**
             * Creates a plain object from a ServiceOptions message. Also converts values to other types if specified.
             * @param message ServiceOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.ServiceOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this ServiceOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for ServiceOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        /** Properties of a MethodOptions. */
        interface IMethodOptions {

            /** MethodOptions deprecated */
            deprecated?: (boolean|null);

            /** MethodOptions idempotencyLevel */
            idempotencyLevel?: (google.protobuf.MethodOptions.IdempotencyLevel|null);

            /** MethodOptions features */
            features?: (google.protobuf.IFeatureSet|null);

            /** MethodOptions uninterpretedOption */
            uninterpretedOption?: (google.protobuf.IUninterpretedOption[]|null);

            /** MethodOptions .database_driver_v1.methodError */
            ".database_driver_v1.methodError"?: (string|null);
        }

        /** Represents a MethodOptions. */
        class MethodOptions implements IMethodOptions {

            /**
             * Constructs a new MethodOptions.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IMethodOptions);

            /** MethodOptions deprecated. */
            public deprecated: boolean;

            /** MethodOptions idempotencyLevel. */
            public idempotencyLevel: google.protobuf.MethodOptions.IdempotencyLevel;

            /** MethodOptions features. */
            public features?: (google.protobuf.IFeatureSet|null);

            /** MethodOptions uninterpretedOption. */
            public uninterpretedOption: google.protobuf.IUninterpretedOption[];

            /**
             * Creates a new MethodOptions instance using the specified properties.
             * @param [properties] Properties to set
             * @returns MethodOptions instance
             */
            public static create(properties?: google.protobuf.IMethodOptions): google.protobuf.MethodOptions;

            /**
             * Encodes the specified MethodOptions message. Does not implicitly {@link google.protobuf.MethodOptions.verify|verify} messages.
             * @param message MethodOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IMethodOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified MethodOptions message, length delimited. Does not implicitly {@link google.protobuf.MethodOptions.verify|verify} messages.
             * @param message MethodOptions message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IMethodOptions, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a MethodOptions message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns MethodOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.MethodOptions;

            /**
             * Decodes a MethodOptions message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns MethodOptions
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.MethodOptions;

            /**
             * Verifies a MethodOptions message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a MethodOptions message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns MethodOptions
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.MethodOptions;

            /**
             * Creates a plain object from a MethodOptions message. Also converts values to other types if specified.
             * @param message MethodOptions
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.MethodOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this MethodOptions to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for MethodOptions
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace MethodOptions {

            /** IdempotencyLevel enum. */
            enum IdempotencyLevel {
                IDEMPOTENCY_UNKNOWN = 0,
                NO_SIDE_EFFECTS = 1,
                IDEMPOTENT = 2
            }
        }

        /** Properties of an UninterpretedOption. */
        interface IUninterpretedOption {

            /** UninterpretedOption name */
            name?: (google.protobuf.UninterpretedOption.INamePart[]|null);

            /** UninterpretedOption identifierValue */
            identifierValue?: (string|null);

            /** UninterpretedOption positiveIntValue */
            positiveIntValue?: (number|Long|null);

            /** UninterpretedOption negativeIntValue */
            negativeIntValue?: (number|Long|null);

            /** UninterpretedOption doubleValue */
            doubleValue?: (number|null);

            /** UninterpretedOption stringValue */
            stringValue?: (Uint8Array|null);

            /** UninterpretedOption aggregateValue */
            aggregateValue?: (string|null);
        }

        /** Represents an UninterpretedOption. */
        class UninterpretedOption implements IUninterpretedOption {

            /**
             * Constructs a new UninterpretedOption.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IUninterpretedOption);

            /** UninterpretedOption name. */
            public name: google.protobuf.UninterpretedOption.INamePart[];

            /** UninterpretedOption identifierValue. */
            public identifierValue: string;

            /** UninterpretedOption positiveIntValue. */
            public positiveIntValue: (number|Long);

            /** UninterpretedOption negativeIntValue. */
            public negativeIntValue: (number|Long);

            /** UninterpretedOption doubleValue. */
            public doubleValue: number;

            /** UninterpretedOption stringValue. */
            public stringValue: Uint8Array;

            /** UninterpretedOption aggregateValue. */
            public aggregateValue: string;

            /**
             * Creates a new UninterpretedOption instance using the specified properties.
             * @param [properties] Properties to set
             * @returns UninterpretedOption instance
             */
            public static create(properties?: google.protobuf.IUninterpretedOption): google.protobuf.UninterpretedOption;

            /**
             * Encodes the specified UninterpretedOption message. Does not implicitly {@link google.protobuf.UninterpretedOption.verify|verify} messages.
             * @param message UninterpretedOption message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IUninterpretedOption, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified UninterpretedOption message, length delimited. Does not implicitly {@link google.protobuf.UninterpretedOption.verify|verify} messages.
             * @param message UninterpretedOption message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IUninterpretedOption, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an UninterpretedOption message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns UninterpretedOption
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.UninterpretedOption;

            /**
             * Decodes an UninterpretedOption message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns UninterpretedOption
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.UninterpretedOption;

            /**
             * Verifies an UninterpretedOption message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an UninterpretedOption message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns UninterpretedOption
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.UninterpretedOption;

            /**
             * Creates a plain object from an UninterpretedOption message. Also converts values to other types if specified.
             * @param message UninterpretedOption
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.UninterpretedOption, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this UninterpretedOption to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for UninterpretedOption
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace UninterpretedOption {

            /** Properties of a NamePart. */
            interface INamePart {

                /** NamePart namePart */
                namePart: string;

                /** NamePart isExtension */
                isExtension: boolean;
            }

            /** Represents a NamePart. */
            class NamePart implements INamePart {

                /**
                 * Constructs a new NamePart.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.UninterpretedOption.INamePart);

                /** NamePart namePart. */
                public namePart: string;

                /** NamePart isExtension. */
                public isExtension: boolean;

                /**
                 * Creates a new NamePart instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns NamePart instance
                 */
                public static create(properties?: google.protobuf.UninterpretedOption.INamePart): google.protobuf.UninterpretedOption.NamePart;

                /**
                 * Encodes the specified NamePart message. Does not implicitly {@link google.protobuf.UninterpretedOption.NamePart.verify|verify} messages.
                 * @param message NamePart message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.UninterpretedOption.INamePart, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified NamePart message, length delimited. Does not implicitly {@link google.protobuf.UninterpretedOption.NamePart.verify|verify} messages.
                 * @param message NamePart message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.UninterpretedOption.INamePart, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a NamePart message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns NamePart
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.UninterpretedOption.NamePart;

                /**
                 * Decodes a NamePart message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns NamePart
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.UninterpretedOption.NamePart;

                /**
                 * Verifies a NamePart message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a NamePart message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns NamePart
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.UninterpretedOption.NamePart;

                /**
                 * Creates a plain object from a NamePart message. Also converts values to other types if specified.
                 * @param message NamePart
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.UninterpretedOption.NamePart, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this NamePart to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for NamePart
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }
        }

        /** Properties of a FeatureSet. */
        interface IFeatureSet {

            /** FeatureSet fieldPresence */
            fieldPresence?: (google.protobuf.FeatureSet.FieldPresence|null);

            /** FeatureSet enumType */
            enumType?: (google.protobuf.FeatureSet.EnumType|null);

            /** FeatureSet repeatedFieldEncoding */
            repeatedFieldEncoding?: (google.protobuf.FeatureSet.RepeatedFieldEncoding|null);

            /** FeatureSet utf8Validation */
            utf8Validation?: (google.protobuf.FeatureSet.Utf8Validation|null);

            /** FeatureSet messageEncoding */
            messageEncoding?: (google.protobuf.FeatureSet.MessageEncoding|null);

            /** FeatureSet jsonFormat */
            jsonFormat?: (google.protobuf.FeatureSet.JsonFormat|null);

            /** FeatureSet enforceNamingStyle */
            enforceNamingStyle?: (google.protobuf.FeatureSet.EnforceNamingStyle|null);

            /** FeatureSet defaultSymbolVisibility */
            defaultSymbolVisibility?: (google.protobuf.FeatureSet.VisibilityFeature.DefaultSymbolVisibility|null);
        }

        /** Represents a FeatureSet. */
        class FeatureSet implements IFeatureSet {

            /**
             * Constructs a new FeatureSet.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFeatureSet);

            /** FeatureSet fieldPresence. */
            public fieldPresence: google.protobuf.FeatureSet.FieldPresence;

            /** FeatureSet enumType. */
            public enumType: google.protobuf.FeatureSet.EnumType;

            /** FeatureSet repeatedFieldEncoding. */
            public repeatedFieldEncoding: google.protobuf.FeatureSet.RepeatedFieldEncoding;

            /** FeatureSet utf8Validation. */
            public utf8Validation: google.protobuf.FeatureSet.Utf8Validation;

            /** FeatureSet messageEncoding. */
            public messageEncoding: google.protobuf.FeatureSet.MessageEncoding;

            /** FeatureSet jsonFormat. */
            public jsonFormat: google.protobuf.FeatureSet.JsonFormat;

            /** FeatureSet enforceNamingStyle. */
            public enforceNamingStyle: google.protobuf.FeatureSet.EnforceNamingStyle;

            /** FeatureSet defaultSymbolVisibility. */
            public defaultSymbolVisibility: google.protobuf.FeatureSet.VisibilityFeature.DefaultSymbolVisibility;

            /**
             * Creates a new FeatureSet instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FeatureSet instance
             */
            public static create(properties?: google.protobuf.IFeatureSet): google.protobuf.FeatureSet;

            /**
             * Encodes the specified FeatureSet message. Does not implicitly {@link google.protobuf.FeatureSet.verify|verify} messages.
             * @param message FeatureSet message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFeatureSet, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FeatureSet message, length delimited. Does not implicitly {@link google.protobuf.FeatureSet.verify|verify} messages.
             * @param message FeatureSet message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFeatureSet, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FeatureSet message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FeatureSet
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FeatureSet;

            /**
             * Decodes a FeatureSet message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FeatureSet
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FeatureSet;

            /**
             * Verifies a FeatureSet message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FeatureSet message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FeatureSet
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FeatureSet;

            /**
             * Creates a plain object from a FeatureSet message. Also converts values to other types if specified.
             * @param message FeatureSet
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FeatureSet, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FeatureSet to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FeatureSet
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace FeatureSet {

            /** FieldPresence enum. */
            enum FieldPresence {
                FIELD_PRESENCE_UNKNOWN = 0,
                EXPLICIT = 1,
                IMPLICIT = 2,
                LEGACY_REQUIRED = 3
            }

            /** EnumType enum. */
            enum EnumType {
                ENUM_TYPE_UNKNOWN = 0,
                OPEN = 1,
                CLOSED = 2
            }

            /** RepeatedFieldEncoding enum. */
            enum RepeatedFieldEncoding {
                REPEATED_FIELD_ENCODING_UNKNOWN = 0,
                PACKED = 1,
                EXPANDED = 2
            }

            /** Utf8Validation enum. */
            enum Utf8Validation {
                UTF8_VALIDATION_UNKNOWN = 0,
                VERIFY = 2,
                NONE = 3
            }

            /** MessageEncoding enum. */
            enum MessageEncoding {
                MESSAGE_ENCODING_UNKNOWN = 0,
                LENGTH_PREFIXED = 1,
                DELIMITED = 2
            }

            /** JsonFormat enum. */
            enum JsonFormat {
                JSON_FORMAT_UNKNOWN = 0,
                ALLOW = 1,
                LEGACY_BEST_EFFORT = 2
            }

            /** EnforceNamingStyle enum. */
            enum EnforceNamingStyle {
                ENFORCE_NAMING_STYLE_UNKNOWN = 0,
                STYLE2024 = 1,
                STYLE_LEGACY = 2
            }

            /** Properties of a VisibilityFeature. */
            interface IVisibilityFeature {
            }

            /** Represents a VisibilityFeature. */
            class VisibilityFeature implements IVisibilityFeature {

                /**
                 * Constructs a new VisibilityFeature.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.FeatureSet.IVisibilityFeature);

                /**
                 * Creates a new VisibilityFeature instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns VisibilityFeature instance
                 */
                public static create(properties?: google.protobuf.FeatureSet.IVisibilityFeature): google.protobuf.FeatureSet.VisibilityFeature;

                /**
                 * Encodes the specified VisibilityFeature message. Does not implicitly {@link google.protobuf.FeatureSet.VisibilityFeature.verify|verify} messages.
                 * @param message VisibilityFeature message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.FeatureSet.IVisibilityFeature, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified VisibilityFeature message, length delimited. Does not implicitly {@link google.protobuf.FeatureSet.VisibilityFeature.verify|verify} messages.
                 * @param message VisibilityFeature message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.FeatureSet.IVisibilityFeature, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a VisibilityFeature message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns VisibilityFeature
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FeatureSet.VisibilityFeature;

                /**
                 * Decodes a VisibilityFeature message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns VisibilityFeature
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FeatureSet.VisibilityFeature;

                /**
                 * Verifies a VisibilityFeature message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a VisibilityFeature message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns VisibilityFeature
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.FeatureSet.VisibilityFeature;

                /**
                 * Creates a plain object from a VisibilityFeature message. Also converts values to other types if specified.
                 * @param message VisibilityFeature
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.FeatureSet.VisibilityFeature, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this VisibilityFeature to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for VisibilityFeature
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }

            namespace VisibilityFeature {

                /** DefaultSymbolVisibility enum. */
                enum DefaultSymbolVisibility {
                    DEFAULT_SYMBOL_VISIBILITY_UNKNOWN = 0,
                    EXPORT_ALL = 1,
                    EXPORT_TOP_LEVEL = 2,
                    LOCAL_ALL = 3,
                    STRICT = 4
                }
            }
        }

        /** Properties of a FeatureSetDefaults. */
        interface IFeatureSetDefaults {

            /** FeatureSetDefaults defaults */
            defaults?: (google.protobuf.FeatureSetDefaults.IFeatureSetEditionDefault[]|null);

            /** FeatureSetDefaults minimumEdition */
            minimumEdition?: (google.protobuf.Edition|null);

            /** FeatureSetDefaults maximumEdition */
            maximumEdition?: (google.protobuf.Edition|null);
        }

        /** Represents a FeatureSetDefaults. */
        class FeatureSetDefaults implements IFeatureSetDefaults {

            /**
             * Constructs a new FeatureSetDefaults.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IFeatureSetDefaults);

            /** FeatureSetDefaults defaults. */
            public defaults: google.protobuf.FeatureSetDefaults.IFeatureSetEditionDefault[];

            /** FeatureSetDefaults minimumEdition. */
            public minimumEdition: google.protobuf.Edition;

            /** FeatureSetDefaults maximumEdition. */
            public maximumEdition: google.protobuf.Edition;

            /**
             * Creates a new FeatureSetDefaults instance using the specified properties.
             * @param [properties] Properties to set
             * @returns FeatureSetDefaults instance
             */
            public static create(properties?: google.protobuf.IFeatureSetDefaults): google.protobuf.FeatureSetDefaults;

            /**
             * Encodes the specified FeatureSetDefaults message. Does not implicitly {@link google.protobuf.FeatureSetDefaults.verify|verify} messages.
             * @param message FeatureSetDefaults message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IFeatureSetDefaults, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified FeatureSetDefaults message, length delimited. Does not implicitly {@link google.protobuf.FeatureSetDefaults.verify|verify} messages.
             * @param message FeatureSetDefaults message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IFeatureSetDefaults, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a FeatureSetDefaults message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns FeatureSetDefaults
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FeatureSetDefaults;

            /**
             * Decodes a FeatureSetDefaults message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns FeatureSetDefaults
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FeatureSetDefaults;

            /**
             * Verifies a FeatureSetDefaults message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a FeatureSetDefaults message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns FeatureSetDefaults
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.FeatureSetDefaults;

            /**
             * Creates a plain object from a FeatureSetDefaults message. Also converts values to other types if specified.
             * @param message FeatureSetDefaults
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.FeatureSetDefaults, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this FeatureSetDefaults to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for FeatureSetDefaults
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace FeatureSetDefaults {

            /** Properties of a FeatureSetEditionDefault. */
            interface IFeatureSetEditionDefault {

                /** FeatureSetEditionDefault edition */
                edition?: (google.protobuf.Edition|null);

                /** FeatureSetEditionDefault overridableFeatures */
                overridableFeatures?: (google.protobuf.IFeatureSet|null);

                /** FeatureSetEditionDefault fixedFeatures */
                fixedFeatures?: (google.protobuf.IFeatureSet|null);
            }

            /** Represents a FeatureSetEditionDefault. */
            class FeatureSetEditionDefault implements IFeatureSetEditionDefault {

                /**
                 * Constructs a new FeatureSetEditionDefault.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.FeatureSetDefaults.IFeatureSetEditionDefault);

                /** FeatureSetEditionDefault edition. */
                public edition: google.protobuf.Edition;

                /** FeatureSetEditionDefault overridableFeatures. */
                public overridableFeatures?: (google.protobuf.IFeatureSet|null);

                /** FeatureSetEditionDefault fixedFeatures. */
                public fixedFeatures?: (google.protobuf.IFeatureSet|null);

                /**
                 * Creates a new FeatureSetEditionDefault instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns FeatureSetEditionDefault instance
                 */
                public static create(properties?: google.protobuf.FeatureSetDefaults.IFeatureSetEditionDefault): google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault;

                /**
                 * Encodes the specified FeatureSetEditionDefault message. Does not implicitly {@link google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault.verify|verify} messages.
                 * @param message FeatureSetEditionDefault message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.FeatureSetDefaults.IFeatureSetEditionDefault, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified FeatureSetEditionDefault message, length delimited. Does not implicitly {@link google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault.verify|verify} messages.
                 * @param message FeatureSetEditionDefault message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.FeatureSetDefaults.IFeatureSetEditionDefault, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a FeatureSetEditionDefault message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns FeatureSetEditionDefault
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault;

                /**
                 * Decodes a FeatureSetEditionDefault message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns FeatureSetEditionDefault
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault;

                /**
                 * Verifies a FeatureSetEditionDefault message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a FeatureSetEditionDefault message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns FeatureSetEditionDefault
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault;

                /**
                 * Creates a plain object from a FeatureSetEditionDefault message. Also converts values to other types if specified.
                 * @param message FeatureSetEditionDefault
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.FeatureSetDefaults.FeatureSetEditionDefault, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this FeatureSetEditionDefault to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for FeatureSetEditionDefault
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }
        }

        /** Properties of a SourceCodeInfo. */
        interface ISourceCodeInfo {

            /** SourceCodeInfo location */
            location?: (google.protobuf.SourceCodeInfo.ILocation[]|null);
        }

        /** Represents a SourceCodeInfo. */
        class SourceCodeInfo implements ISourceCodeInfo {

            /**
             * Constructs a new SourceCodeInfo.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.ISourceCodeInfo);

            /** SourceCodeInfo location. */
            public location: google.protobuf.SourceCodeInfo.ILocation[];

            /**
             * Creates a new SourceCodeInfo instance using the specified properties.
             * @param [properties] Properties to set
             * @returns SourceCodeInfo instance
             */
            public static create(properties?: google.protobuf.ISourceCodeInfo): google.protobuf.SourceCodeInfo;

            /**
             * Encodes the specified SourceCodeInfo message. Does not implicitly {@link google.protobuf.SourceCodeInfo.verify|verify} messages.
             * @param message SourceCodeInfo message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.ISourceCodeInfo, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified SourceCodeInfo message, length delimited. Does not implicitly {@link google.protobuf.SourceCodeInfo.verify|verify} messages.
             * @param message SourceCodeInfo message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.ISourceCodeInfo, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a SourceCodeInfo message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns SourceCodeInfo
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.SourceCodeInfo;

            /**
             * Decodes a SourceCodeInfo message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns SourceCodeInfo
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.SourceCodeInfo;

            /**
             * Verifies a SourceCodeInfo message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a SourceCodeInfo message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns SourceCodeInfo
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.SourceCodeInfo;

            /**
             * Creates a plain object from a SourceCodeInfo message. Also converts values to other types if specified.
             * @param message SourceCodeInfo
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.SourceCodeInfo, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this SourceCodeInfo to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for SourceCodeInfo
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace SourceCodeInfo {

            /** Properties of a Location. */
            interface ILocation {

                /** Location path */
                path?: (number[]|null);

                /** Location span */
                span?: (number[]|null);

                /** Location leadingComments */
                leadingComments?: (string|null);

                /** Location trailingComments */
                trailingComments?: (string|null);

                /** Location leadingDetachedComments */
                leadingDetachedComments?: (string[]|null);
            }

            /** Represents a Location. */
            class Location implements ILocation {

                /**
                 * Constructs a new Location.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.SourceCodeInfo.ILocation);

                /** Location path. */
                public path: number[];

                /** Location span. */
                public span: number[];

                /** Location leadingComments. */
                public leadingComments: string;

                /** Location trailingComments. */
                public trailingComments: string;

                /** Location leadingDetachedComments. */
                public leadingDetachedComments: string[];

                /**
                 * Creates a new Location instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Location instance
                 */
                public static create(properties?: google.protobuf.SourceCodeInfo.ILocation): google.protobuf.SourceCodeInfo.Location;

                /**
                 * Encodes the specified Location message. Does not implicitly {@link google.protobuf.SourceCodeInfo.Location.verify|verify} messages.
                 * @param message Location message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.SourceCodeInfo.ILocation, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Location message, length delimited. Does not implicitly {@link google.protobuf.SourceCodeInfo.Location.verify|verify} messages.
                 * @param message Location message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.SourceCodeInfo.ILocation, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Location message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns Location
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.SourceCodeInfo.Location;

                /**
                 * Decodes a Location message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns Location
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.SourceCodeInfo.Location;

                /**
                 * Verifies a Location message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates a Location message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Location
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.SourceCodeInfo.Location;

                /**
                 * Creates a plain object from a Location message. Also converts values to other types if specified.
                 * @param message Location
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.SourceCodeInfo.Location, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Location to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for Location
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }
        }

        /** Properties of a GeneratedCodeInfo. */
        interface IGeneratedCodeInfo {

            /** GeneratedCodeInfo annotation */
            annotation?: (google.protobuf.GeneratedCodeInfo.IAnnotation[]|null);
        }

        /** Represents a GeneratedCodeInfo. */
        class GeneratedCodeInfo implements IGeneratedCodeInfo {

            /**
             * Constructs a new GeneratedCodeInfo.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IGeneratedCodeInfo);

            /** GeneratedCodeInfo annotation. */
            public annotation: google.protobuf.GeneratedCodeInfo.IAnnotation[];

            /**
             * Creates a new GeneratedCodeInfo instance using the specified properties.
             * @param [properties] Properties to set
             * @returns GeneratedCodeInfo instance
             */
            public static create(properties?: google.protobuf.IGeneratedCodeInfo): google.protobuf.GeneratedCodeInfo;

            /**
             * Encodes the specified GeneratedCodeInfo message. Does not implicitly {@link google.protobuf.GeneratedCodeInfo.verify|verify} messages.
             * @param message GeneratedCodeInfo message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IGeneratedCodeInfo, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified GeneratedCodeInfo message, length delimited. Does not implicitly {@link google.protobuf.GeneratedCodeInfo.verify|verify} messages.
             * @param message GeneratedCodeInfo message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IGeneratedCodeInfo, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a GeneratedCodeInfo message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns GeneratedCodeInfo
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.GeneratedCodeInfo;

            /**
             * Decodes a GeneratedCodeInfo message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns GeneratedCodeInfo
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.GeneratedCodeInfo;

            /**
             * Verifies a GeneratedCodeInfo message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a GeneratedCodeInfo message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns GeneratedCodeInfo
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.GeneratedCodeInfo;

            /**
             * Creates a plain object from a GeneratedCodeInfo message. Also converts values to other types if specified.
             * @param message GeneratedCodeInfo
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.GeneratedCodeInfo, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this GeneratedCodeInfo to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };

            /**
             * Gets the default type url for GeneratedCodeInfo
             * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
             * @returns The default type url
             */
            public static getTypeUrl(typeUrlPrefix?: string): string;
        }

        namespace GeneratedCodeInfo {

            /** Properties of an Annotation. */
            interface IAnnotation {

                /** Annotation path */
                path?: (number[]|null);

                /** Annotation sourceFile */
                sourceFile?: (string|null);

                /** Annotation begin */
                begin?: (number|null);

                /** Annotation end */
                end?: (number|null);

                /** Annotation semantic */
                semantic?: (google.protobuf.GeneratedCodeInfo.Annotation.Semantic|null);
            }

            /** Represents an Annotation. */
            class Annotation implements IAnnotation {

                /**
                 * Constructs a new Annotation.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: google.protobuf.GeneratedCodeInfo.IAnnotation);

                /** Annotation path. */
                public path: number[];

                /** Annotation sourceFile. */
                public sourceFile: string;

                /** Annotation begin. */
                public begin: number;

                /** Annotation end. */
                public end: number;

                /** Annotation semantic. */
                public semantic: google.protobuf.GeneratedCodeInfo.Annotation.Semantic;

                /**
                 * Creates a new Annotation instance using the specified properties.
                 * @param [properties] Properties to set
                 * @returns Annotation instance
                 */
                public static create(properties?: google.protobuf.GeneratedCodeInfo.IAnnotation): google.protobuf.GeneratedCodeInfo.Annotation;

                /**
                 * Encodes the specified Annotation message. Does not implicitly {@link google.protobuf.GeneratedCodeInfo.Annotation.verify|verify} messages.
                 * @param message Annotation message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encode(message: google.protobuf.GeneratedCodeInfo.IAnnotation, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Encodes the specified Annotation message, length delimited. Does not implicitly {@link google.protobuf.GeneratedCodeInfo.Annotation.verify|verify} messages.
                 * @param message Annotation message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                public static encodeDelimited(message: google.protobuf.GeneratedCodeInfo.IAnnotation, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Annotation message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns Annotation
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.GeneratedCodeInfo.Annotation;

                /**
                 * Decodes an Annotation message from the specified reader or buffer, length delimited.
                 * @param reader Reader or buffer to decode from
                 * @returns Annotation
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.GeneratedCodeInfo.Annotation;

                /**
                 * Verifies an Annotation message.
                 * @param message Plain object to verify
                 * @returns `null` if valid, otherwise the reason why it is not
                 */
                public static verify(message: { [k: string]: any }): (string|null);

                /**
                 * Creates an Annotation message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Annotation
                 */
                public static fromObject(object: { [k: string]: any }): google.protobuf.GeneratedCodeInfo.Annotation;

                /**
                 * Creates a plain object from an Annotation message. Also converts values to other types if specified.
                 * @param message Annotation
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                public static toObject(message: google.protobuf.GeneratedCodeInfo.Annotation, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Annotation to JSON.
                 * @returns JSON object
                 */
                public toJSON(): { [k: string]: any };

                /**
                 * Gets the default type url for Annotation
                 * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
                 * @returns The default type url
                 */
                public static getTypeUrl(typeUrlPrefix?: string): string;
            }

            namespace Annotation {

                /** Semantic enum. */
                enum Semantic {
                    NONE = 0,
                    SET = 1,
                    ALIAS = 2
                }
            }
        }

        /** SymbolVisibility enum. */
        enum SymbolVisibility {
            VISIBILITY_UNSET = 0,
            VISIBILITY_LOCAL = 1,
            VISIBILITY_EXPORT = 2
        }
    }
}
