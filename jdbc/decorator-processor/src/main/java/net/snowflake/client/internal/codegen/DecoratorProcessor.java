package net.snowflake.client.internal.codegen;

import java.io.IOException;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import javax.annotation.processing.AbstractProcessor;
import javax.annotation.processing.RoundEnvironment;
import javax.annotation.processing.SupportedAnnotationTypes;
import javax.annotation.processing.SupportedSourceVersion;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.Element;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.Modifier;
import javax.lang.model.element.TypeElement;
import javax.lang.model.element.TypeParameterElement;
import javax.lang.model.element.VariableElement;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.ElementFilter;
import javax.tools.Diagnostic;
import javax.tools.JavaFileObject;

/**
 * Generates decorator classes for {@link JdbcBoundary}-annotated impls. Reads the interfaces
 * directly from the class's {@code implements} clause, skipping internal ones.
 */
@SupportedAnnotationTypes("net.snowflake.client.internal.codegen.JdbcBoundary")
@SupportedSourceVersion(SourceVersion.RELEASE_8)
public class DecoratorProcessor extends AbstractProcessor {

  private static final Set<String> SKIPPED_INTERFACES = new HashSet<>();
  private static final Set<String> PASS_THROUGH_INTERFACES = new HashSet<>();
  private static final Set<String> SKIPPED_METHODS = new HashSet<>();

  static {
    SKIPPED_INTERFACES.add("net.snowflake.client.internal.util.DelegatingWrapper");
    SKIPPED_INTERFACES.add("java.sql.Wrapper");
    SKIPPED_INTERFACES.add("java.io.Serializable");
    SKIPPED_INTERFACES.add("java.lang.AutoCloseable");
    SKIPPED_INTERFACES.add("java.io.Closeable");

    // Interfaces whose own methods are internal plumbing but whose public super-interfaces must
    // still be delegated: the generated decorator implements the super-interfaces, not this one.
    PASS_THROUGH_INTERFACES.add(
        "net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection");
    PASS_THROUGH_INTERFACES.add(
        "net.snowflake.client.internal.api.implementation.statement.InternalStatement");

    SKIPPED_METHODS.add("unwrap");
    SKIPPED_METHODS.add("isWrapperFor");
  }

  private static final String DECORATOR_PACKAGE = "net.snowflake.client.internal.api.decorator";

  @Override
  public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment roundEnv) {
    for (Element element : roundEnv.getElementsAnnotatedWith(JdbcBoundary.class)) {
      if (!(element instanceof TypeElement)) {
        continue;
      }
      try {
        generateDecorator((TypeElement) element);
      } catch (IOException e) {
        processingEnv
            .getMessager()
            .printMessage(
                Diagnostic.Kind.ERROR,
                "Failed to generate decorator: " + e.getMessage(),
                element);
      }
    }
    return true;
  }

  private void generateDecorator(TypeElement implElement) throws IOException {
    String implQualifiedName = implElement.getQualifiedName().toString();
    String implSimpleName = implElement.getSimpleName().toString();
    String implPackage = implQualifiedName.substring(0, implQualifiedName.lastIndexOf('.'));
    String decoratorName = "Decorated" + implSimpleName;

    // Read interfaces from the implements clause, expanding pass-through interfaces into their
    // delegatable super-interfaces and filtering out internal/marker ones.
    List<TypeElement> interfaceElements = new ArrayList<>();
    Set<String> seenInterfaces = new HashSet<>();
    for (TypeMirror iface : implElement.getInterfaces()) {
      collectDelegatableInterfaces(iface, interfaceElements, seenInterfaces);
    }

    if (interfaceElements.isEmpty()) {
      processingEnv
          .getMessager()
          .printMessage(
              Diagnostic.Kind.WARNING,
              "@JdbcBoundary: no delegatable interfaces found on " + implSimpleName,
              implElement);
      return;
    }

    // Index impl methods for parameter-name resolution and throws/@NoTelemetry detection. Use
    // getAllMembers (not getEnclosedElements) so *inherited* methods are indexed too — otherwise
    // delegateThrowsChecked() misses that an inherited delegate declares no checked exception and
    // emits a dead `catch (SQLException)`. getAllMembers resolves overrides; skip abstract members
    // so only concrete impl signatures are kept.
    Map<String, ExecutableElement> implMethods = new LinkedHashMap<>();
    for (ExecutableElement m :
        ElementFilter.methodsIn(
            processingEnv.getElementUtils().getAllMembers(implElement))) {
      if (!m.getModifiers().contains(Modifier.ABSTRACT)) {
        implMethods.put(methodSignature(m), m);
      }
    }

    // Collect all abstract methods from the interfaces
    Map<String, MethodInfo> methods = new LinkedHashMap<>();
    for (TypeElement iface : interfaceElements) {
      collectMethods(iface, methods);
    }

    // Resolve parameter names from impl where possible
    for (Map.Entry<String, MethodInfo> entry : methods.entrySet()) {
      ExecutableElement implMethod = implMethods.get(entry.getKey());
      if (implMethod != null) {
        entry.getValue().implMethod = implMethod;
      }
    }

    // Collect imports
    TreeSet<String> imports = new TreeSet<>();
    imports.add("java.sql.SQLException");
    imports.add("javax.annotation.Generated");
    imports.add(DECORATOR_PACKAGE + ".AbstractDecorator");
    imports.add(DECORATOR_PACKAGE + ".Telemetry");
    for (TypeElement iface : interfaceElements) {
      String ifaceName = iface.getQualifiedName().toString();
      if (!ifaceName.startsWith("java.lang.")) {
        imports.add(ifaceName);
      }
    }
    for (MethodInfo mi : methods.values()) {
      addImports(imports, mi);
    }

    // Generate source in the same package as the impl
    String qualifiedDecoratorName = implPackage + "." + decoratorName;
    JavaFileObject sourceFile =
        processingEnv.getFiler().createSourceFile(qualifiedDecoratorName, implElement);

    try (PrintWriter w = new PrintWriter(sourceFile.openWriter())) {
      w.println("package " + implPackage + ";");
      w.println();

      for (String imp : imports) {
        // Skip same-package imports
        if (imp.startsWith(implPackage + ".")
            && !imp.substring(implPackage.length() + 1).contains(".")) {
          continue;
        }
        w.println("import " + imp + ";");
      }
      w.println();

      w.println("@Generated(\"" + DecoratorProcessor.class.getName() + "\")");
      w.print("public final class " + decoratorName);
      w.println();
      w.print("    extends AbstractDecorator<" + implSimpleName + ">");
      w.println();
      w.print("    implements ");
      for (int i = 0; i < interfaceElements.size(); i++) {
        if (i > 0) {
          w.print(", ");
        }
        w.print(interfaceElements.get(i).getSimpleName());
      }
      w.println(" {");
      w.println();

      w.println(
          "  public "
              + decoratorName
              + "("
              + implSimpleName
              + " delegate, Telemetry telemetry) {");
      w.println("    super(delegate, telemetry);");
      w.println("  }");

      boolean needsSneakyThrow = false;
      for (MethodInfo mi : methods.values()) {
        // A hot accessor that declares SQLException translates directly (throw translateHot(e))
        // and never sneaky-throws; every other sneaky-using shape is covered by the two clauses.
        if (!declaresSqlException(mi.method) || (!isHotAccessor(mi) && delegateThrowsChecked(mi))) {
          needsSneakyThrow = true;
          break;
        }
      }

      for (MethodInfo mi : methods.values()) {
        w.println();
        writeMethod(w, mi);
      }

      if (needsSneakyThrow) {
        w.println();
        w.println("  @SuppressWarnings(\"unchecked\")");
        w.println("  private static <E extends Throwable>"
            + " RuntimeException sneakyThrow(Throwable t) throws E {");
        w.println("    throw (E) t;");
        w.println("  }");
      }

      w.println("}");
    }
  }

  /**
   * Expands one interface from the {@code implements} clause into the interfaces to delegate: skips
   * wrapper/marker interfaces; for a pass-through interface delegates its public super-interfaces
   * instead; otherwise adds it directly. Deduplicates via {@code seen}.
   */
  private void collectDelegatableInterfaces(
      TypeMirror iface, List<TypeElement> out, Set<String> seen) {
    if (iface.getKind() != TypeKind.DECLARED) {
      return;
    }
    TypeElement ifaceElement = (TypeElement) ((DeclaredType) iface).asElement();
    String name = ifaceElement.getQualifiedName().toString();
    if (SKIPPED_INTERFACES.contains(name)) {
      return;
    }
    if (PASS_THROUGH_INTERFACES.contains(name)) {
      for (TypeMirror superIface : ifaceElement.getInterfaces()) {
        collectDelegatableInterfaces(superIface, out, seen);
      }
      return;
    }
    if (seen.add(name)) {
      out.add(ifaceElement);
    }
  }

  private void collectMethods(TypeElement iface, Map<String, MethodInfo> methods) {
    for (ExecutableElement method : ElementFilter.methodsIn(iface.getEnclosedElements())) {
      String name = method.getSimpleName().toString();
      if (SKIPPED_METHODS.contains(name)) {
        continue;
      }
      String sig = methodSignature(method);
      if (!methods.containsKey(sig)) {
        methods.put(sig, new MethodInfo(method));
      }
    }
    for (TypeMirror superIface : iface.getInterfaces()) {
      if (superIface.getKind() == TypeKind.DECLARED) {
        TypeElement superElement = (TypeElement) ((DeclaredType) superIface).asElement();
        String superName = superElement.getQualifiedName().toString();
        if (!SKIPPED_INTERFACES.contains(superName)) {
          collectMethods(superElement, methods);
        }
      }
    }
  }

  private String methodSignature(ExecutableElement method) {
    StringBuilder sb = new StringBuilder();
    sb.append(method.getSimpleName());
    sb.append('(');
    for (int i = 0; i < method.getParameters().size(); i++) {
      if (i > 0) {
        sb.append(',');
      }
      sb.append(
          processingEnv
              .getTypeUtils()
              .erasure(method.getParameters().get(i).asType())
              .toString());
    }
    sb.append(')');
    return sb.toString();
  }

  private void addImports(TreeSet<String> imports, MethodInfo mi) {
    addTypeImport(imports, mi.method.getReturnType());
    for (VariableElement param : mi.method.getParameters()) {
      addTypeImport(imports, param.asType());
    }
    for (TypeMirror thrown : mi.method.getThrownTypes()) {
      addTypeImport(imports, thrown);
    }
    for (TypeParameterElement tp : mi.method.getTypeParameters()) {
      for (TypeMirror bound : tp.getBounds()) {
        addTypeImport(imports, bound);
      }
    }
  }

  private void addTypeImport(TreeSet<String> imports, TypeMirror type) {
    if (type.getKind() == TypeKind.DECLARED) {
      DeclaredType dt = (DeclaredType) type;
      TypeElement te = (TypeElement) dt.asElement();
      String qualified = te.getQualifiedName().toString();
      if (!qualified.startsWith("java.lang.")) {
        imports.add(qualified);
      }
      for (TypeMirror arg : dt.getTypeArguments()) {
        addTypeImport(imports, arg);
      }
    }
  }

  private void writeMethod(PrintWriter w, MethodInfo mi) {
    ExecutableElement method = mi.method;
    String returnType = typeToString(method.getReturnType());
    String methodName = method.getSimpleName().toString();
    boolean isVoid = method.getReturnType().getKind() == TypeKind.VOID;
    boolean noTelemetry =
        mi.implMethod != null && mi.implMethod.getAnnotation(NoTelemetry.class) != null;
    boolean needsTryCatch = delegateThrowsChecked(mi);

    ExecutableElement nameSource = mi.implMethod != null ? mi.implMethod : method;

    w.println("  @Override");
    w.print("  public " + typeParameters(method) + returnType + " " + methodName + "(");

    List<? extends VariableElement> params = method.getParameters();
    List<? extends VariableElement> nameParams = nameSource.getParameters();
    for (int i = 0; i < params.size(); i++) {
      if (i > 0) {
        w.print(", ");
      }
      w.print(typeToString(params.get(i).asType()) + " " + nameParams.get(i).getSimpleName());
    }
    w.print(") " + buildThrowsClause(method) + "{");
    w.println();

    StringBuilder args = new StringBuilder();
    for (int i = 0; i < params.size(); i++) {
      if (i > 0) {
        args.append(", ");
      }
      args.append(nameParams.get(i).getSimpleName());
    }

    String delegateCall = "delegate." + methodName + "(" + args + ")";

    // Hot accessor (per-row / per-column, @NoTelemetry): call the delegate directly instead of
    // through a captured lambda + invoke(), skipping the per-call telemetry overhead. Exceptions
    // are still translated at this boundary.
    if (noTelemetry) {
      writeHotAccessor(w, mi, delegateCall, isVoid);
      w.println("  }");
      return;
    }

    // Op-name uses the *declaring* JDBC interface, so inherited methods read as e.g.
    // "Statement.execute" even on the PreparedStatement decorator.
    String opName = declaringInterfaceName(method) + "." + methodName;
    String callPrefix = "\"" + opName + "\", ";

    // run/call throw SQLException. If this override's clause can't absorb that (e.g. Connection's
    // setClientInfo declares only the narrower SQLClientInfoException), catch and sneaky-throw so
    // we neither widen the clause nor swallow the error.
    boolean wrapOuterSql = !declaresSqlException(method);
    String indent = wrapOuterSql ? "      " : "    ";
    if (wrapOuterSql) {
      w.println("    try {");
    }
    if (needsTryCatch) {
      writeTryCatchLambda(w, delegateCall, callPrefix, isVoid, indent);
    } else {
      writeDirectLambda(w, delegateCall, callPrefix, isVoid, indent);
    }
    if (wrapOuterSql) {
      w.println("    } catch (SQLException e) { throw sneakyThrow(e); }");
    }

    w.println("  }");
  }

  /**
   * Emits the body of a hot accessor: a direct delegate call wrapped only in exception translation,
   * with no captured lambda, {@code invoke()} hop, or success-path {@link ThreadLocal}. A runtime
   * failure is translated via {@code translateHot}; the result is thrown directly when the
   * signature declares {@code SQLException}, else sneaky-thrown. When the signature can't carry
   * {@code SQLException} but the delegate declares it, that checked exception is passed through
   * (sneaky-thrown) untranslated; when the signature declares it, the delegate's exception simply
   * propagates.
   */
  private void writeHotAccessor(
      PrintWriter w, MethodInfo mi, String delegateCall, boolean isVoid) {
    boolean declaresSql = declaresSqlException(mi.method);
    String runtimeThrow =
        declaresSql ? "throw translateHot(e);" : "throw sneakyThrow(translateHot(e));";
    w.println("    try {");
    w.println("      " + (isVoid ? "" : "return ") + delegateCall + ";");
    w.print("    } catch (RuntimeException e) { " + runtimeThrow + " }");
    if (!declaresSql && delegateThrowsChecked(mi)) {
      w.print(" catch (SQLException e) { throw sneakyThrow(e); }");
    }
    w.println();
  }

  /** True for a {@link NoTelemetry}-marked method, matching {@code noTelemetry} in writeMethod. */
  private boolean isHotAccessor(MethodInfo mi) {
    return mi.implMethod != null && mi.implMethod.getAnnotation(NoTelemetry.class) != null;
  }

  private void writeDirectLambda(
      PrintWriter w, String delegateCall, String callPrefix, boolean isVoid, String indent) {
    if (isVoid) {
      w.println(indent + "run(" + callPrefix + "() -> " + delegateCall + ");");
    } else {
      w.println(indent + "return call(" + callPrefix + "() -> " + delegateCall + ");");
    }
  }

  private void writeTryCatchLambda(
      PrintWriter w, String delegateCall, String callPrefix, boolean isVoid, String indent) {
    if (isVoid) {
      w.println(indent + "run(" + callPrefix + "() -> {");
      w.println(indent + "  try { " + delegateCall + "; }");
    } else {
      w.println(indent + "return call(" + callPrefix + "() -> {");
      w.println(indent + "  try { return " + delegateCall + "; }");
    }
    w.println(indent + "  catch (SQLException e) { throw sneakyThrow(e); }");
    w.println(indent + "});");
  }

  /**
   * Simple name of the interface that declares {@code method}, so inherited methods keep their
   * declaring interface (e.g. {@code Statement}) rather than the implementing one.
   */
  private String declaringInterfaceName(ExecutableElement method) {
    return method.getEnclosingElement().getSimpleName().toString();
  }

  /**
   * Renders the {@code throws} clause (trailing space, empty when none). Mirrors the interface
   * exactly rather than assuming {@code SQLException}, since an override may only narrow it.
   */
  private String buildThrowsClause(ExecutableElement method) {
    List<? extends TypeMirror> thrown = method.getThrownTypes();
    if (thrown.isEmpty()) {
      return "";
    }
    StringBuilder sb = new StringBuilder("throws ");
    for (int i = 0; i < thrown.size(); i++) {
      if (i > 0) {
        sb.append(", ");
      }
      sb.append(typeToString(thrown.get(i)));
    }
    sb.append(' ');
    return sb.toString();
  }

  /**
   * True when the method's clause can absorb a {@code SQLException} (declares it or a supertype).
   * False for an empty or narrower clause (e.g. {@code SQLClientInfoException}), so the body must
   * catch and sneaky-throw.
   */
  private boolean declaresSqlException(ExecutableElement method) {
    TypeElement sqlException =
        processingEnv.getElementUtils().getTypeElement("java.sql.SQLException");
    if (sqlException == null) {
      return true;
    }
    for (TypeMirror thrown : method.getThrownTypes()) {
      if (processingEnv.getTypeUtils().isSubtype(sqlException.asType(), thrown)) {
        return true;
      }
    }
    return false;
  }

  private boolean delegateThrowsChecked(MethodInfo mi) {
    ExecutableElement source = mi.implMethod != null ? mi.implMethod : mi.method;
    for (TypeMirror thrown : source.getThrownTypes()) {
      if (thrown.getKind() != TypeKind.DECLARED) {
        continue;
      }
      TypeElement thrownElement = (TypeElement) ((DeclaredType) thrown).asElement();
      String name = thrownElement.getQualifiedName().toString();
      if (!name.equals("java.lang.RuntimeException")
          && !isSubtypeOf(thrownElement, "java.lang.RuntimeException")
          && !name.equals("java.lang.Error")
          && !isSubtypeOf(thrownElement, "java.lang.Error")) {
        return true;
      }
    }
    return false;
  }

  private boolean isSubtypeOf(TypeElement element, String superTypeName) {
    TypeElement superType = processingEnv.getElementUtils().getTypeElement(superTypeName);
    return superType != null
        && processingEnv.getTypeUtils().isSubtype(element.asType(), superType.asType());
  }

  /**
   * Renders a method's own type parameters with bounds (e.g. {@code "<T> "}), trailing space,
   * empty for non-generic methods.
   */
  private String typeParameters(ExecutableElement method) {
    List<? extends TypeParameterElement> typeParams = method.getTypeParameters();
    if (typeParams.isEmpty()) {
      return "";
    }
    StringBuilder sb = new StringBuilder("<");
    for (int i = 0; i < typeParams.size(); i++) {
      if (i > 0) {
        sb.append(", ");
      }
      TypeParameterElement tp = typeParams.get(i);
      sb.append(tp.getSimpleName());
      List<? extends TypeMirror> bounds = tp.getBounds();
      // A single java.lang.Object bound is the implicit default — omit it.
      if (bounds.size() == 1
          && bounds.get(0).toString().equals("java.lang.Object")) {
        continue;
      }
      for (int b = 0; b < bounds.size(); b++) {
        sb.append(b == 0 ? " extends " : " & ");
        sb.append(typeToString(bounds.get(b)));
      }
    }
    sb.append("> ");
    return sb.toString();
  }

  private String typeToString(TypeMirror type) {
    if (type.getKind() == TypeKind.DECLARED) {
      DeclaredType dt = (DeclaredType) type;
      TypeElement te = (TypeElement) dt.asElement();
      String simpleName = te.getSimpleName().toString();
      List<? extends TypeMirror> typeArgs = dt.getTypeArguments();
      if (typeArgs.isEmpty()) {
        return simpleName;
      }
      StringBuilder sb = new StringBuilder(simpleName);
      sb.append('<');
      for (int i = 0; i < typeArgs.size(); i++) {
        if (i > 0) {
          sb.append(", ");
        }
        sb.append(typeToString(typeArgs.get(i)));
      }
      sb.append('>');
      return sb.toString();
    }
    return type.toString();
  }

  private static class MethodInfo {
    final ExecutableElement method;
    ExecutableElement implMethod;

    MethodInfo(ExecutableElement method) {
      this.method = method;
    }
  }
}
